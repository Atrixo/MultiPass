(b_uebung)
(  model: Z1 3 axis)
(T1  Fräser Ø3 (lang)  Makera    D=3. - ZMIN=-2. - Schaftfraser)
(   Z Origin Set To   : Stock Top )
(   X Offset   : 0. mm )
(   Y Offset   : 0. mm )
G90 G94
G17
G21
( )
(Öffnung)
T1 M6
S9000 M3
G54
G0 X5 Y5
Z10
G1 Z2 F150 ; anfahren
X15 Y5 F400
Z-2 F60
X20
G1 Z12 F150
G53 Z0
X30 Y30
G1 Z10 X31 Y31
Y40
Z15
M5
M30
