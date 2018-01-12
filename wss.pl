#!/usr/bin/perl -w
#
# wss	Estimate the working set size (WSS) for a process on Linux.
#
# USAGE: wss PID duration(s)
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

sub usage {
	die <<USAGE_END;
USAGE: wss [options] PID duration(s)
	-i secs            # repeat output every secs seconds
   eg,
	wss 181 0.01       # measure PID 181 for 10 milliseconds
	wss -i 1 181 0.01  # ... and print output every second
USAGE_END
}

my $interval = 0;
GetOptions(
	'interval|i=f'  => \$interval,
) or usage();

if (@ARGV < 2 || $ARGV[0] eq "-h" || $ARGV[0] eq "--help") {
	usage();
	exit;
}
my $pid = $ARGV[0];
my $duration = $ARGV[1];
my $clear_ref = "/proc/$pid/clear_refs";
my $smaps = "/proc/$pid/smaps";
my ($rss, $pss, $referenced);
my $metric;

printf "%10s %10s %10s\n", "RSS(MB)", "PSS(MB)", "Ref(MB)";
while (1) {
	open CLEAR, ">$clear_ref" or die "ERROR: can't open $clear_ref (older kernel?): $!";
	print CLEAR "1";
	close CLEAR;

	select(undef, undef, undef, $duration);

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

	if ($interval) {
		sleep($interval);
	} else {
		last;
	}
}
