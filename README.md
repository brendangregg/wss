# Working Set Size (WSS) Tools for Linux

These are experimental tools for doing working set size estimation, using different Linux facilities. See WARNINGs.

## wss

This tool resets the PG\_referenced page flags via /proc/PID/clear\_refs, then checks referenced memory after a duration. Eg:

<pre>
# <b>./wss.pl 5922 0.01</b>
Watching PID 5922 page references during 0.01 seconds...
   RSS(MB)    PSS(MB)    Ref(MB)
    101.07     100.10       5.11
</pre>

The output shows that the process had 101 Mbytes of RSS (main memory), and during 0.01 seconds only 5.11 Mbytes (worth of pages) was touched (read/written).

Columns:

- `RSS(MB)`: Resident Set Size (Mbytes). The main memory size.
- `PSS(MB)`: Proportional Set Size (Mbytes). Accounting for shared pages.
- `Ref(MB)`: Referenced (Mbytes) during the specified duration. This is the working set size metric.

USAGE:

<pre>
# <b>./wss.pl -h</b>
Unknown option: h
USAGE: wss [options] PID duration(s)
	-C         # show cumulative output every duration(s)
	-s secs    # take duration(s) snapshots after secs pauses
	-d secs    # total duration of measuremnt (for -s or -C)
	-P steps   # profile run, starting with duration(s)
   eg,
	wss 181 0.01       # measure PID 181 WSS for 10 milliseconds
	wss 181 5          # measure PID 181 WSS for 5 seconds (same overhead)
	wss -C 181 5       # show PID 181 growth every 5 seconds
	wss -Cd 10 181 1   # PID 181 growth each second for 10 seconds total
	wss -s 1 181 0.01  # show a 10 ms WSS snapshot every 1 second
	wss -s 0 181 1     # measure WSS every 1 second (not cumulative)
	wss -P 10 181 0.01 # 10 step power-of-2 profile, starting with 0.01s
</pre>

WARNINGs:

This tool uses /proc/PID/clear_refs and /proc/PID/smaps, and does
pause the target process for some milliseconds while address maps are read.
This can cause a short burst of latency for your application. For processes
with a lot of RSS (>100 Gbytes), the pause may be 1 second or longer. This
also activates some old kernel code that may not have been used in your
environment before, and which mucks with page flags: I'd guess there is a
risk of an undiscovered kernel panic (the Linux mm community should know
whether my guess is justified or not, if you want an expert opinion). Test in
a lab environment for your kernel versions, and consider this experimental:
use at your on risk.
