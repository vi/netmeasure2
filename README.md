# netmeasure2

Tool to run experiments to measure networking quality.

Usage:

1. Run server: `netmeasure2 serve 0.0.0.0:12345 --min-packetdelay-us 2 --bwlimit 200000`
2. On another host, run the test battery: `netmeasure2 battery 192.168.0.1:12345 --big -o results.json`. There are two modes: 15-megabyte small battery and 300-megabyte big battery.
3. Analyse the results: `netmeasure2 showbat results.json`. There is overall score at the end.

There is a pre-built release on Github Releases.
