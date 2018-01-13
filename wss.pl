#!/usr/bin/perl -w
#
# wss	Estimate the working set size (WSS) for a process on Linux.
#
# USAGE: wss [options] PID duration(s)
#    eg,
#        wss 181 0.01	# measure PID 181 WSS for 10 milliseconds
#        wss -h		# full USAGE
#
# COLUMNS:
#	- RSS(MB): Resident Set Size (Mbytes). The main memory size.
#	- PSS(MB): Proportional Set Size (Mbytes). Accounting for shared pages.
#	- Ref(MB): Referenced (Mbytes) during the specified duration.
#	           This is the working set size metric.
#
# I could add more columns, but that's what pmap -X is for.
#
# WARNING: This tool uses /proc/PID/clear_refs and /proc/PID/smaps, and does
# pause the target process for some milliseconds while address maps are read.
# This can cause a short burst of latency for your application. For processes
# with a lot of RSS (>100 Gbytes), the pause may be 1 second or longer. This
# also activates some old kernel code that may not have been used in your
# environment before, and which mucks with page flags: I'd guess there is a
# risk of an undiscovered kernel panic (the Linux mm community should know
# whether my guess is justified or not, if you want an expert opinion). Test in
# a lab environment for your kernel versions, and consider this experimental:
# use at your on risk.
#
# Copyright 2018 Netflix, Inc.
# Licensed under the Apache License, Version 2.0 (the "License")
#
# 10-Jan-2018	Brendan Gregg	Created this.

use strict;
use Getopt::Long;
$| = 1;

sub usage {
	die <<USAGE_END;
USAGE: wss [options] PID duration(s)
	-C         # show cumulative output every duration(s)
	-s secs    # take duration(s) snapshots after secs pauses
	-d secs    # total duration of measuremnt (for -s or -C)
	-P steps   # profile run (cumulative), from duration(s)
   eg,
	wss 181 0.01       # measure PID 181 WSS for 10 milliseconds
	wss 181 5          # measure PID 181 WSS for 5 seconds (same overhead)
	wss -C 181 5       # show PID 181 growth every 5 seconds
	wss -Cd 10 181 1   # PID 181 growth each second for 10 seconds total
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
GetOptions(
	'snapshot|s=f'  => \$snapshot,
	'duration|d=f'  => \$totalsecs,
	'cumulative|C'  => \$cumulative,
	'profile|P=i'  => \$profile,
) or usage();
my $pid = $ARGV[0];
my $duration = $ARGV[1];

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
my @profilesecs = ();
if ($profile) {
	my $d = $duration;
	for (my $i = 0; $i < $profile; $i++) {
		push(@profilesecs, $d);
		$d *= 2;
	}
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
printf "%-8s ", "Dur(s)" if $profile;
printf "%10s %10s %10s\n", "RSS(MB)", "PSS(MB)", "Ref(MB)";

### main
my ($rss, $pss, $referenced);
my $metric;
my $time = 0;
my $firstreset = 0;

while (1) {
	if (not $firstreset or $snapshot != -1) {
		open CLEAR, ">$clear_ref" or die "ERROR: can't open $clear_ref (older kernel?): $!";
		print CLEAR "1";
		close CLEAR;
		$firstreset = 1;
	}

	my $sleep = $duration;
	if ($profile) {
		$sleep = shift @profilesecs;
		last unless defined $sleep;
	}
	select(undef, undef, undef, $sleep);
	$time += $duration;

	$rss = $pss = $referenced = 0;
	open SMAPS, $smaps or die "ERROR: can't open $smaps: $!";
	while (my $line = <SMAPS>) {
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
	close SMAPS;

	printf "%-8.3f ", $sleep if $profile;
	printf "%10.2f %10.2f %10.2f\n", $rss / 1024, $pss / 1024, $referenced / 1024;

	if ($snapshot != -1) {
		select(undef, undef, undef, $snapshot);
		$time += $snapshot;
	} elsif (not $cumulative and not $profile) {
		last;
	}

	if ($time >= $totalsecs) {
		last;
	}
}
