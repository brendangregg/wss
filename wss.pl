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
	-s secs    # show a duration(s) snapshot every secs
	-d secs    # total duration of measuremnt (for -s or -C)
   eg,
	wss 181 0.01       # measure PID 181 WSS for 10 milliseconds
	wss 181 5          # measure PID 181 WSS for 5 seconds (same overhead)
	wss -C 181 5       # show PID 181 growth every 5 seconds
	wss -Cd 10 181 1   # PID 181 growth each second for 10 seconds total
	wss -s 1 181 0.01  # show a 10 ms WSS snapshot every 1 second
USAGE_END
}

my $snapshot = 0;
my $totalsecs = 999999999;
my $cumulative = 0;
GetOptions(
	'snapshot|s=f'  => \$snapshot,
	'duration|d=f'  => \$totalsecs,
	'cumulative|C'  => \$cumulative,
) or usage();

if (@ARGV < 2 || $ARGV[0] eq "-h" || $ARGV[0] eq "--help") {
	usage();
	exit;
}
if ($cumulative and $snapshot) {
	print STDERR "ERROR: Can't use -C and -s (doesn't make much sense). Exiting.\n";
	exit;
}
my $pid = $ARGV[0];
my $duration = $ARGV[1];
my $clear_ref = "/proc/$pid/clear_refs";
my $smaps = "/proc/$pid/smaps";
my ($rss, $pss, $referenced);
my $metric;
my $reset = 0;
my $time = 0;

# headers
if ($cumulative) {
	printf "Watching PID $pid page references grow, output every $duration seconds...\n";
} elsif ($snapshot) {
	printf "Watching PID $pid page references for $duration seconds, once every $snapshot seconds...\n";
} else {
	printf "Watching PID $pid page references during $duration seconds...\n";
}
printf "%10s %10s %10s\n", "RSS(MB)", "PSS(MB)", "Ref(MB)";

# main
while (1) {
	if (not $cumulative or not $reset) {
		open CLEAR, ">$clear_ref" or die "ERROR: can't open $clear_ref (older kernel?): $!";
		print CLEAR "1";
		close CLEAR;
		$reset = 1;
	}

	select(undef, undef, undef, $duration);
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

	printf "%10.2f %10.2f %10.2f\n", $rss / 1024, $pss / 1024, $referenced / 1024;

	if ($snapshot) {
		sleep($snapshot);
		$time += $snapshot;
	} elsif (not $cumulative) {
		last;
	}

	if ($time >= $totalsecs) {
		last;
	}
}
