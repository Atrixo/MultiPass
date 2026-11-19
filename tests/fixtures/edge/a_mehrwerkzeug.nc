(a_mehrwerkzeug)
(Maschine)
(  model: Z1 3 axis)
(T1  Flat End Mill 2mm  Makera    D=2. - ZMIN=-1. - Schaftfraser)
(T2  Ball Mill 1mm)
(   Z Origin Set To   : Stock Top )
(   Toolpath Z Min from Stock Top: -1. mm )
(   X Offset   : 0. mm )
(   Y Offset   : 0. mm )
G90 G94
G17
G21

(Tasche)
T1M6
M400
S12000 M3
G54
G0 X0 Y0
Z15.
G1 Z5. F100
X10. Y0. F500
Y10.
G1 Z0.5 F100
Z-1. F80
X20. F300
G2 X25. Y15. I5. J0.
X30. Y20. I2.5 J2.5
G1 Z0.8
Z5.
X40. Y40. Z10. F600 ; leerfahrt
G1 X41 Y41
N100 G1 X42 Y42 Z12
G0 Z20.
X50 Y50
G1 Z2. F200
X55. Y55.
Z-0.5
X60
G28
G1 X70 Y70 Z5 F400
Z30
X80 Y80
Z20
M5
(Profil)
t2 m06
S10000 M3
G0 X0. Y0.
G1 Z0.5 F100
X1.23456 Y-0.00004 F250
Y.5
X-.25
Z-.5 F50
G3 X0 Y0 R1
G1 Z10.
X5. Y5.
G1 Z0.9 F100
X6
M5
M400
G28
M30
%
