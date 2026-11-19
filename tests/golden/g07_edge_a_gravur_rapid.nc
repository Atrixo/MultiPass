(Zusammengefuegt mit MultiPass 1.0 am 2026-11-19 01:16)
(Reihenfolge:)
(  1. a_mehrwerkzeug.nc -> T1 Flat End Mill 2mm Makera, T2 Ball Mill 1mm)
(  2. 4_T16_Gravur.cnc -> T3 Single Flute Engraving Metal 30 deg*.3mm)

(a_mehrwerkzeug)
(Maschine)
(  model: Z1 3 axis)
(   Z Origin Set To   : Stock Top )
(   Toolpath Z Min from Stock Top: -1. mm )
(   X Offset   : 0. mm )
(   Y Offset   : 0. mm )
G90 G94
G17
G21

(Werkzeuge:)
(T1  Flat End Mill 2mm Makera)
(T2  Ball Mill 1mm)
(T3  Single Flute Engraving Metal 30 deg*.3mm)

(===== Teil 1: a_mehrwerkzeug.nc =====)
(Tasche)
T1M6
M400
S12000 M3
G54
G0 X0 Y0
Z15.
G0 Z5.
G0 X10. Y0.
G0 Y10.
G1 Z0.5 F100
Z-1. F80
X20. F300
G2 X25. Y15. I5. J0.
X30. Y20. I2.5 J2.5
G0 Z0.8
G0 Z5.
G0 X40. Y40. Z10.
G0 X41. Y41.
N100 G1 X42 Y42 Z12 F600.
G0 Z20.
X50 Y50
G0 Z2.
G0 X55. Y55.
G1 Z-0.5 F200.
X60
G28
G1 X70 Y70 Z5 F400
G0 Z30.
G0 X80. Y80.
G0 Z20.
M5
(Profil)
T2 m06
S10000 M3
G0 X0. Y0.
G1 Z0.5 F100
X1.23456 Y-0.00004 F250
Y.5
X-.25
Z-.5 F50
G3 X0 Y0 R1
G0 Z10.
G0 X5. Y5.
G1 Z0.9 F100
X6
M5
M400
G28

(===== Teil 2: 4_T16_Gravur.cnc =====)
(T16 Gravur 0.0)
T3M6 
(8)
M400
M851 S100
S13000 M3
G54
G0 X37.25 Y36.376
Z15.
G0 Z5.
G1 Z-0.3 F50
Y43.99 F100
X37.2 Y44.69
X37.02 Y45.52
X36.63 Y46.47
X36.17 Y47.2
X35.46 Y47.95
X34.77 Y48.46
X33.85 Y48.91
X32.86 Y49.18
X31.99 Y49.25
X11.49 Y49.22
X10.98 Y49.15
X10.31 Y48.97
X9.68 Y48.71
X9.08 Y48.37
X8.67 Y48.06
X8.17 Y47.59
X7.73 Y47.06
X7.37 Y46.47
X7.15 Y46.01
X6.93 Y45.36
X6.82 Y44.86
X6.75 Y44.17
X6.76 Y11.66
X6.85 Y10.98
X7.03 Y10.31
X7.29 Y9.68
X7.54 Y9.23
X7.94 Y8.67
X8.41 Y8.17
X8.94 Y7.73
X9.53 Y7.37
X10.15 Y7.09
X10.64 Y6.93
X11.14 Y6.82
X11.83 Y6.75
X32.34 Y6.76
X33.02 Y6.85
X33.69 Y7.03
X34.32 Y7.29
X34.77 Y7.54
X35.33 Y7.94
X35.83 Y8.41
X36.27 Y8.94
X36.63 Y9.53
X36.91 Y10.15
X37.11 Y10.81
X37.22 Y11.49
X37.25 Y12.01
Y36.376
G0 Z5.
G0 X20.54 Y39.53
G1 Z-0.3 F50
X20.78 Y39.96 F100
G0 Z5.
G0 X22. Y28.8
G1 Z-0.3 F50
Y21.4 F100
G0 Z5.
G0 Z15.
(When using Fusion for Personal Use, the feedrate of rapid)
(moves is reduced to match the feedrate of cutting moves,)
(which can increase machining time. Unrestricted rapid moves)
(are available with a Fusion Subscription.)

(T16 Gravur 0.6)
G0 X18.3 Y28.4
G1 Z15. F100
G0 Z5.
G1 Z-0.9 F50
X20.7 F100
G0 Z5.
G0 X20. Y27.2
G1 Z-0.9 F50
X18.8 Y22.8 F100
G0 Z5.
G0 X18.6 Y26.8
G1 Z-0.9 F50
X17.6 Y23.4 F100
G0 Z5.
G0 X24. Y27.2
G1 Z-0.9 F50
X25.2 Y22.8 F100
G0 Z5.
G0 X25.4 Y26.8
G1 Z-0.9 F50
X26.4 Y23.4 F100
G0 Z5.
G0 X23.3 Y28.4
G1 Z-0.9 F50
X25.7 F100
G0 Z5.
G0 X23.6 Y34.4
G1 Z-0.9 F50
X24. Y33. F100
G0 Z5.
G0 X20.4 Y34.4
G1 Z-0.9 F50
X20. Y33. F100
G0 Z5.
G0 Z15.
(When using Fusion for Personal Use, the feedrate of rapid)
(moves is reduced to match the feedrate of cutting moves,)
(which can increase machining time. Unrestricted rapid moves)
(are available with a Fusion Subscription.)

(T16 Gravur 1.2)
G0 X17.08 Y36.3
G1 Z15. F100
G0 Z5.
G1 Z-1.5 F50
X14.75 Y38. F100
G0 Z5.
G0 X16.89 Y35.37
G1 Z-1.5 F50
X14.4 Y36.23 F100
G0 Z5.
G0 X17. Y34.54
G1 Z-1.5 F50
X14.61 Y34.67 F100
G0 Z5.
G0 X17.41 Y33.91
G1 Z-1.5 F50
X15.38 Y33.48 F100
G0 Z5.
G0 X26.59 Y33.91
G1 Z-1.5 F50
X28.62 Y33.48 F100
G0 Z5.
G0 X27. Y34.54
G1 Z-1.5 F50
X29.39 Y34.67 F100
G0 Z5.
G0 X27.11 Y35.37
G1 Z-1.5 F50
X29.6 Y36.23 F100
G0 Z5.
G0 X26.92 Y36.3
G1 Z-1.5 F50
X29.25 Y38. F100
G0 Z5.
G0 Z15.
(When using Fusion for Personal Use, the feedrate of rapid)
(moves is reduced to match the feedrate of cutting moves,)
(which can increase machining time. Unrestricted rapid moves)
(are available with a Fusion Subscription.)

(T16 Gravur 1.8)
G0 X13.99 Y39.54
G1 Z15. F100
G0 Z5.
G1 Z-2.1 F50
X11.14 Y42.11 F100
G0 Z5.
G0 X13.07 Y37.78
G1 Z-2.1 F50
X9.76 Y39.47 F100
G0 Z5.
G0 X13.01 Y35.82
G1 Z-2.1 F50
X9.66 Y36.53 F100
G0 Z5.
G0 X13.58 Y34.08
G1 Z-2.1 F50
X10.89 Y33.94 F100
G0 Z5.
G0 X14.57 Y32.73
G1 Z-2.1 F50
X12.85 Y32.18 F100
G0 Z5.
G0 X29.43 Y32.73
G1 Z-2.1 F50
X31.15 Y32.18 F100
G0 Z5.
G0 X30.42 Y34.08
G1 Z-2.1 F50
X33.11 Y33.94 F100
G0 Z5.
G0 X30.99 Y35.82
G1 Z-2.1 F50
X34.34 Y36.53 F100
G0 Z5.
G0 X30.93 Y37.78
G1 Z-2.1 F50
X34.24 Y39.47 F100
G0 Z5.
G0 X30.01 Y39.54
G1 Z-2.1 F50
X32.86 Y42.11 F100
G0 Z5.
G0 Z15.
M5
M400
M852
G28

M30
