# Working Set Size (WSS) Tools for Linux

These are experimental tools for doing working set size estimation, using different Linux facilities. See WARNINGs.

Main website: http://www.brendangregg.com/wss.html

Tools:

- **wss.pl**: For Linux 2.6.22+. Uses the referenced page flag for a page-based WSS estimation.
- **wss-v1**: For Linux 4.3+, and small processes. Uses the idle page flag for a page-based WSS estimation.
- **wss-v2**: For Linux 4.3+, and large processes. Uses the idle page flag for a page-based WSS estimation.

## wss.pl (referenced page flag)

This tool should work on Linux 2.26.22+, although with caveats described below. It resets the PG\_referenced page flags via /proc/PID/clear\_refs, then checks referenced memory after a duration. Eg:

<pre>
# <b>./wss.pl 23593 0.1</b>
Watching PID 23593 page references during 0.1 seconds...
Est(s)     RSS(MB)    PSS(MB)    Ref(MB)
0.100       201.18     200.10      10.41
</pre>

The output shows that the process had 201 Mbytes of RSS (main memory), and during 0.1 seconds only 10.41 Mbytes (worth of pages) was touched (read/written).

Columns:

- `Est(s)`:  Estimated WSS measurement duration: this accounts for delays with setting and reading pagemap data, which inflates the intended sleep duration.
- `RSS(MB)`: Resident Set Size (Mbytes). The main memory size.
- `PSS(MB)`: Proportional Set Size (Mbytes). Accounting for shared pages.
- `Ref(MB)`: Referenced (Mbytes) during the specified duration. This is the working set size metric.
- `Dur(s)`:  Full duration of measurement (seconds), from beginning to set page flags to completing reading them.
- `Slp(s)`:  Total sleep time.

USAGE:

<pre>
# <b>./wss.pl -h</b>
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
</pre>

WARNINGs:

This tool uses /proc/PID/clear\_refs and /proc/PID/smaps, which can
cause slightly higher application latency (eg, 10%) while the kernel walks page
structures. For large processes (> 100 Gbytes) this duration of
higher latency can last over 1 second, during which this tool is consuming
system CPU time. Consider these overheads. This
also resets the referenced flag, which might confuse the kernel as to which
pages to reclaim, especially if swapping is active. This also activates some
old kernel code that may not have been used in your environment before, and
which modifies page flags: I'd guess there is a risk of an undiscovered
kernel panic (the Linux mm community may be able to say how real this risk
is). Test in a lab environment for your kernel versions, and consider this
experimental: use at your on risk.

## wss-v1.c (idle page flag: small process)

This is a proof-of-concept tool that uses idle page tracking, which was added to Linux 4.3. This is considered safer than modifying the referenced page flag, since the referenced page flag may confuse the kernel reclaim code, especially if the system is swapping.

This version of this tool walks page structures one by one, and is suited for small processes only. On large processes (>100 Gbytes), this tool can take several minutes to write. See wss-v2.c, which uses page data snapshots and is much faster for large processes (50x), as well as wss.pl, which is even faster (although uses the referenced page flag).

Here is some example output, comparing this tool to the earlier wss.pl:

<pre>
# <b>./wss-v1 33583 0.01</b>
Watching PID 33583 page references during 0.01 seconds...
Est(s)     Ref(MB)
0.055        10.00

# <b>./wss.pl 33583 0.01</b>
Watching PID 33583 page references during 0.01 seconds...
Est(s)     RSS(MB)    PSS(MB)    Ref(MB)
0.011        21.07      20.10      10.03
</pre>

The output shows that that process referenced 10 Mbytes of data (this is correct: it's a synthetic workload).

Columns:

- `Est(s)`:  Estimated WSS measurement duration: this accounts for delays with setting and reading pagemap data, which inflates the intended sleep duration.
- `Ref(MB)`: Referenced (Mbytes) during the specified duration. This is the working set size metric.

WARNINGs:

This tool sets and reads process page flags, which for large
processes (> 100 Gbytes) can take several minutes (use wss-v2 for those
instead). During that time, this tool consumes one CPU, and the application
may experience slightly higher latency (eg, 5%). Consider these overheads.
Also, this is activating some new kernel code added in Linux 4.3 that you
may have never executed before. As is the case for any such code, there is
the risk of undiscovered kernel panics (I have no specific reason to worry,
just being paranoid). Test in a lab environment for your kernel versions,
and consider this experimental: use at your own risk.

## wss-v2.c (idle page flag: large process)

This is a proof-of-concept tool that uses idle page tracking, which was added to Linux 4.3. This is considered safer than modifying the referenced page flag, since the referenced page flag may confuse the kernel reclaim code, especially if the system is swapping.

This version of this tool takes a snapshot of the system's idle page flags, which speeds up analysis of large processes, but not small ones. See wss-v1.c, which may be faster for small processes, as well as wss.pl, which is even faster (although uses the referenced page flag).

Here is some example output, comparing this tool to wss-v1 (which runs much slower), and the earlier wss.pl:

<pre>
# <b>./wss-v2 27357 0.01</b>
Watching PID 27357 page references during 0.01 seconds...
Est(s)     Ref(MB)
0.806        15.00

# <b>./wss-v1 27357 0.01</b>
Watching PID 27357 page references during 0.01 seconds...
Est(s)     Ref(MB)
44.571       16.00

# <b>./wss.pl 27357 0.01</b>
Watching PID 27357 page references during 0.01 seconds...
Est(s)     RSS(MB)    PSS(MB)    Ref(MB)
0.080     20001.12   20000.14      15.03
</pre>

The output shows that that process referenced 15 Mbytes of data (this is correct: it's a synthetic workload).

Columns:

- `Est(s)`:  Estimated WSS measurement duration: this accounts for delays with setting and reading pagemap data, which inflates the intended sleep duration.
- `Ref(MB)`: Referenced (Mbytes) during the specified duration. This is the working set size metric.

WARNINGs:

This tool sets and reads system and process page flags, which can
take over one second of CPU time, during which application may experience
slightly higher latency (eg, 5%). Consider these overheads. Also, this is
activating some new kernel code added in Linux 4.3 that you may have never
executed before. As is the case for any such code, there is the risk of
undiscovered kernel panics (I have no specific reason to worry, just being
paranoid). Test in a lab environment for your kernel versions, and consider
this experimental: use at your own risk.
