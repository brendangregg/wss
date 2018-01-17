#!/usr/bin/perl -w
#
# wss.pl	Estimate the working set size (WSS) for a process on Linux.
#
# This uses /proc/PID/clear_refs and works on older Linux's (2.6.22+),
# however, comes with warnings below. See its companion tools, wss-v1.c and
# wss-v2.c, which uses # the newer idle page tracking from Linux 4.3+, however,
# they are much slower to run (as described in the tools). The WSS measured
# here is page-based.
#
# http://www.brendangregg.com/wss.pl
#
# USAGE: wss [options] PID duration(s)
#    eg,
#        wss 181 0.01   # measure PID 181 WSS for 10 milliseconds
#        wss -h         # full USAGE
#
# COLUMNS:
#	- Est(s):  Estimated WSS measurement duration: this accounts for delays
#	           with setting and reading pagemap data, which inflates the
#	           intended sleep duration.
#	- RSS(MB): Resident Set Size (Mbytes). The main memory size.
#	- PSS(MB): Proportional Set Size (Mbytes). Accounting for shared pages.
#	- Ref(MB): Referenced (Mbytes) during the specified duration.
#	           This is the working set size metric.
#	- Dur(s):  Full duration of measurement (seconds), from beginning to
#	           set page flags to completing reading them.
#	- Slp(s):  Total sleep time.
#
# I could add more columns, but that's what pmap -X is for.
#
# WARNING: This tool uses /proc/PID/clear_refs and /proc/PID/smaps, which can
# cause slightly higher application latency while the kernel walks process page
# structures. For large processes (> 100 Gbytes) this overhead duration of
# higher latency can last over 1 second (the system time of this tool). This
# also resets the referenced flag, which might confuse the kernel as to which
# pages to reclaim, especially if swapping is active. This also activates some
# old kernel code that may not have been used in your environment before, and
# which modifies page flags: I'd guess there is a risk of an undiscovered
# kernel panic (the Linux mm community may be able to say how real this risk
# is). Test in a lab environment for your kernel versions, and consider this
# experimental: use at your on risk.
#
# About the duration columns: here's how you think this tool should work:
#
#	1. reset referenced page flags for a process (instantaneous)
#	2. sleep for the duration
#	3. read referenced page flags (instantaneous)
#
# Here's what actually happens:
#
# 	1. begin resetting page flags for a process
# 	2. [... CPU time passes ...]
# 	3. page flag reset completes
# 	4. sleep for a duration
# 	5. begin reading page flags
# 	6. [... CPU time passes ...]
# 	7. read complete
#
# So we get our WSS page count, but does it reflect just the sleep duration?
# No, the working set is being touched during stages 2 and 6, which inflate
# the sleep duration. Those stages for large processes (>100 Gbytes) can take
# over 500 ms of CPU time, and so a 10 ms target duration can really be
# reflecting 100s of ms of memory changes.
#
# To inform the end user of this duration inflation, this tool provides an
# estimated duration, measuring from the midpoint of stage 2 to the midpoint of
# stage 6. For small processes, this estimated duration will likely equal the
# intended duration. But for large processes, it will show the inflated time.
#
# There is also an experimental, undocumented (in USAGE), and dangerous option
# for dealing with the duration inflation in a different way: --pausetarget,
# which will pause the target process (SIGSTOP) and only let it run for the
# duration of the measurements (using: SIGCONT -> measure -> SIGSTOP). Large
# processes (> 100 Gbytes) could be paused for 1 second or longer, creating
# severe application latency. This option is deactivated in the code, and
# only exists for experimentation in a lab environment.
#
# Copyright 2018 Netflix, Inc.
# Licensed under the Apache License, Version 2.0 (the "License")
#
# 10-Jan-2018	Brendan Gregg	Created this.

use strict;
use Getopt::Long;
use Time::HiRes;
$| = 1;

sub usage {
	die <<USAGE_END;
USAGE: wss [options] PID duration(s)
	-C         # show cumulative output every duration(s)
	-s secs    # take duration(s) snapshots after secs pauses
	-d secs    # total duration of measuremnt (for -s or -C)
	-P steps   # profile run (cumulative), from duration(s)
	-t         # show additional timestamp columns
   eg,
	wss 181 0.01       # measure PID 181 WSS for 10 milliseconds
	wss 181 5          # measure PID 181 WSS for 5 seconds (same overhead)
	wss -C 181 5       # show PID 181 growth every 5 seconds
	wss -C -d 10 181 1 # PID 181 growth each second for 10 seconds total
	wss -s 1 181 0.01  # show a 10 ms WSS snapshot every 1 second
	wss -s 0 181 1     # measure WSS every 1 second (not cumulative)
	wss -P 10 181 0.01 # 10 step power-of-2 profile, starting with 0.01s
USAGE_END
}

### options
my $snapshot = -1;
my $totalsecs = 999999999;
my $cumulative = 0;
my $profile = 0;
my $moretimes = 0;
my $pausetarget = 0;
GetOptions(
	'snapshot|s=f'  => \$snapshot,
	'duration|d=f'  => \$totalsecs,
	'cumulative|C'  => \$cumulative,
	'profile|P=i'  => \$profile,
	'moretimes|t'  => \$moretimes,
	'pausetarget'  => \$pausetarget,
	'help|h' => 'usage',
) or usage();
my $pid = $ARGV[0];
my $duration = $ARGV[1];
if ($pausetarget) {
	print STDERR "--pausetarget disabled (too dangerous). See code.\n";
	exit;
	# if you comment this out, be aware you're sending SIGSTOP/SIGCONTs
	# to the target process, which will pause it, creating latency. If
	# wss.pl crashes or is SIGKILL'd, then the target process can be left
	# in SIGSTOP and will need to be SIGCONT'd manually.
}
if (@ARGV < 2 || $ARGV[0] eq "-h" || $ARGV[0] eq "--help") {
	usage();
	exit;
}
if ((!!$cumulative + ($snapshot != -1) + !!$profile) > 1) {
	print STDERR "ERROR: Can't combine -C, -s, and P. Exiting.\n";
	exit;
}
if ($duration < 0.001) {
	print STDERR "ERROR: Duration too short. Exiting.\n";
	exit;
}
my $clear_ref = "/proc/$pid/clear_refs";
my $smaps = "/proc/$pid/smaps";
my @profilesecs = ($duration);
my $d;
if ($profile) {
	$d = $duration;
	for (my $i = 0; $i < $profile - 1; $i++) {
		push(@profilesecs, $d);
		$d *= 2;
	}
}
if ($pausetarget) {
	shift(@profilesecs);
	push(@profilesecs, $d);
}

### headers
if ($profile) {
	printf "Watching PID $pid page references grow, profile beginning with $duration seconds, $profile steps...\n";
} elsif ($cumulative) {
	printf "Watching PID $pid page references grow, output every $duration seconds...\n";
} elsif ($snapshot != -1) {
	if ($snapshot == 0) {
		printf "Watching PID $pid page references for every $duration seconds...\n";
	} else {
		printf "Watching PID $pid page references for $duration seconds, repeating after $snapshot second pauses...\n";
	}
} else {
	printf "Watching PID $pid page references during $duration seconds...\n";
}
printf "%-7s %-7s ", "Slp(s)", "Dur(s)" if $moretimes;
printf "%-7s %10s %10s %10s\n", "Est(s)", "RSS(MB)", "PSS(MB)", "Ref(MB)";

### main
my ($rss, $pss, $referenced);
my ($ts0, $ts1, $ts2, $ts3, $ts4, $ts5);
my ($settime, $sleeptime, $readtime, $durtime, $esttime);
my $metric;
my $firstreset = 0;
$sleeptime = 0;

### cleanup
sub cleanup {
	kill -CONT, $pid;
	exit 0;
}
if ($pausetarget) {
	$SIG{INT} = 'cleanup';    # Ctrl-C
	$SIG{QUIT} = 'cleanup';   # Ctrl-\
	$SIG{TERM} = 'cleanup';   # TERM
}

$ts0 = Time::HiRes::gettimeofday();
while (1) {
	# reset referenced flags
	if (not $firstreset or $snapshot != -1 or $pausetarget) {
		kill -STOP, $pid if $pausetarget;
		open CLEAR, ">$clear_ref" or die "ERROR: can't open $clear_ref (older kernel?): $!";
		$ts1 = Time::HiRes::gettimeofday();
		print CLEAR "1";
		close CLEAR;
		$ts2 = Time::HiRes::gettimeofday();
		$settime = $ts2 - $ts1;
		$firstreset = 1;
	}

	# pause
	my $sleep = $duration;
	if ($profile) {
		$sleep = shift @profilesecs;
		last unless defined $sleep;
	}
	kill -CONT, $pid if $pausetarget;
	$ts3 = Time::HiRes::gettimeofday();
	select(undef, undef, undef, $sleep);
	$ts4 = Time::HiRes::gettimeofday();
	kill -STOP, $pid if $pausetarget;

	# read referenced counts
	$rss = $pss = $referenced = 0;
	open SMAPS, $smaps or die "ERROR: can't open $smaps: $!";
	# slurp smaps quickly to minimize unwanted WSS growth during reading:
	my @smaps = <SMAPS>;
	$ts5 = Time::HiRes::gettimeofday();
	close SMAPS;
	kill -CONT, $pid if ($pausetarget and $snapshot != -1);
	foreach my $line (@smaps) {
		if ($line =~ /^Rss:/) {
			$metric = \$rss;
		} elsif ($line =~ /^Pss:/) {
			$metric = \$pss;
		} elsif ($line =~ /^Referenced:/) {
			$metric = \$referenced;
		} else {
			next;
		}
		# now pay the split cost, after filtering out most lines:
		my ($junk1, $kbytes, $junk2) = split ' ', $line;
		$$metric += $kbytes;
	}

	# time calculations
	if ($snapshot != -1 or $pausetarget) {
		$sleeptime = $ts4 - $ts3;
	} else {
		$sleeptime += $ts4 - $ts3;
	}
	$readtime = $ts5 - $ts4;
	$durtime = $ts5 - $ts1;
	if ($pausetarget) {
		$esttime = $ts4 - $ts3;
	} else {
		$esttime = $durtime - ($settime / 2) - ($readtime / 2);
	}

	# output
	printf "%-7.3f %-7.3f ", $sleeptime, $durtime if $moretimes;
	printf "%-7.3f %10.2f %10.2f %10.2f\n", $esttime,
	    $rss / 1024, $pss / 1024, $referenced / 1024;

	# snopshot sleeps
	if ($snapshot != -1) {
		select(undef, undef, undef, $snapshot);
	} elsif (not $cumulative and not $profile) {
		last;
	}

	if ($ts5 - $ts0 >= $totalsecs) {
		last;
	}
}

kill -CONT, $pid if $pausetarget;
