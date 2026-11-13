dcl_resource_texture2darray (float,float,float,float) t110
dcl_resource_texture2darray (float,float,float,float) t111
dcl_resource_texture2darray (float,float,float,float) t112
dcl_resource_texture2darray (float,float,float,float) t113
dcl_resource_texture2darray (float,float,float,float) t114
dcl_resource_texture2darray (float,float,float,float) t115
dcl_resource_texture2darray (float,float,float,float) t116
dcl_resource_texture2darray (float,float,float,float) t117
// EDCAS ground detail: x511 colour, z511 relief, w511 fade distance (0 = 1), y511 debug views, x514 ground type
ld_indexable(texture1d)(float,float,float,float) {c}.xyzw, l(511, 0, 0, 0), t120.xyzw
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(514, 0, 0, 0), t120.xyzw
mov {q}.z, {j}.x
mov {m}.w, {j}.y
lt {h}.xy, l(0.000100, 0.000100, 0.000000, 0.000000), {c}.xzxx
ge {h}.zw, l(0.000000, 0.000000, 0.000000, 0.000000), {c}.xxxz
or {h}.xy, {h}.xyxx, {h}.zwzz
or {h}.x, {h}.x, {h}.y
ne {h}.y, {c}.y, l(0.000000)
or {h}.x, {h}.x, {h}.y
if_nz {h}.x
exp {a}.x, cb2[32].w
div {a}.y, cb2[0].x, {a}.x
mul {b}.xyzw, {a}.yyyy, l(0.031250, 0.125000, 0.500000, 2.000000)
round_ne {b}.xyzw, {b}.xyzw
max {b}.xyzw, {b}.xyzw, l(1.000000, 1.000000, 1.000000, 1.000000)
mul {b}.xyzw, {b}.xyzw, {a}.xxxx
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(513, 0, 0, 0), t120.xyzw
eq {i}.zw, {j}.yyyz, l(0.000000, 0.000000, 0.000000, 0.000000)
movc {i}.zw, {i}.zzzw, l(1.000000, 1.000000, 1.000000, 1.000000), {j}.yyyz
mov {i}.xy, l(0.000000, 0.000000, 0.000000, 0.000000)
eq {k}.z, {j}.w, l(0.000000)
movc {k}.z, {k}.z, l(1.000000), {j}.w
dp3 {a}.z, v3.xyzx, v3.xyzx
sqrt {a}.z, {a}.z
lt {a}.w, l(0.000000), {c}.w
movc {a}.w, {a}.w, {c}.w, l(1.000000)
div {a}.z, {a}.z, {a}.w
mad_sat {a}.zw, {a}.zzzz, l(0.000000, 0.000000, -0.005882353, -0.001428571), l(0.000000, 0.000000, 1.470588, 1.428571)
mul {i}.w, {i}.w, {a}.z
mul {k}.z, {k}.z, {a}.w
mov {k}.x, l(0.000000)
// EDCAS crust mask: {m}.x
mov {m}.x, l(0.000000)
mov {m}.y, {a}.w
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(514, 0, 0, 0), t120.xyzw
ge {f}.x, {j}.x, l(1.500000)
lt {f}.y, {j}.x, l(3.500000)
and {f}.x, {f}.x, {f}.y
if_nz {f}.x
add {g}.xyz, v3.xyzx, -cb2[12].xyzx
mov {m}.x, l(0.500000)
dp3 {n}.x, {g}.xyzx, l(-0.000242190, -0.000660840, 0.001368027, 0.000000)
dp3 {n}.y, {g}.xyzx, l(-0.000872392, 0.001194643, 0.000422640, 0.000000)
dp3 {n}.z, {g}.xyzx, l(-0.001243841, -0.000709213, -0.000562797, 0.000000)
round_ni {o}.xyz, {n}.xyzx
add {n}.xyz, {n}.xyzx, -{o}.xyzx
mad {p}.xyz, {n}.xyzx, l(-2.000000, -2.000000, -2.000000, 0.000000), l(3.000000, 3.000000, 3.000000, 0.000000)
mul {r}.xyz, {n}.xyzx, {n}.xyzx
mul {p}.xyz, {p}.xyzx, {r}.xyzx
ftoi {o}.xyz, {o}.xyzx
iadd {s}.xyzw, {o}.xxxx, l(0, 1, 0, 1)
imul null, {s}.xyzw, {s}.xyzw, l(0x8da6b343, 0x8da6b343, 0x8da6b343, 0x8da6b343)
iadd {t}.xyzw, {o}.yyyy, l(0, 0, 1, 1)
imul null, {t}.xyzw, {t}.xyzw, l(0xd8163841, 0xd8163841, 0xd8163841, 0xd8163841)
xor {s}.xyzw, {s}.xyzw, {t}.xyzw
iadd {t}.xy, {o}.zzzz, l(0, 1, 0, 0)
imul null, {t}.xy, {t}.xyxx, l(0xcb1ab31f, 0xcb1ab31f, 0, 0)
xor {v}.xyzw, {s}.xyzw, {t}.xxxx
xor {u}.xyzw, {s}.xyzw, {t}.yyyy
iadd {v}.xyzw, {v}.xyzw, l(0x2545f491, 0x2545f491, 0x2545f491, 0x2545f491)
ushr {h}.xyzw, {v}.xyzw, l(16, 16, 16, 16)
xor {v}.xyzw, {v}.xyzw, {h}.xyzw
imul null, {v}.xyzw, {v}.xyzw, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0x7feb352d)
ushr {h}.xyzw, {v}.xyzw, l(15, 15, 15, 15)
xor {v}.xyzw, {v}.xyzw, {h}.xyzw
imul null, {v}.xyzw, {v}.xyzw, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0x846ca68b)
ushr {h}.xyzw, {v}.xyzw, l(16, 16, 16, 16)
xor {v}.xyzw, {v}.xyzw, {h}.xyzw
utof {v}.xyzw, {v}.xyzw
mul {v}.xyzw, {v}.xyzw, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 2.328306437E-10)
iadd {u}.xyzw, {u}.xyzw, l(0x2545f491, 0x2545f491, 0x2545f491, 0x2545f491)
ushr {h}.xyzw, {u}.xyzw, l(16, 16, 16, 16)
xor {u}.xyzw, {u}.xyzw, {h}.xyzw
imul null, {u}.xyzw, {u}.xyzw, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0x7feb352d)
ushr {h}.xyzw, {u}.xyzw, l(15, 15, 15, 15)
xor {u}.xyzw, {u}.xyzw, {h}.xyzw
imul null, {u}.xyzw, {u}.xyzw, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0x846ca68b)
ushr {h}.xyzw, {u}.xyzw, l(16, 16, 16, 16)
xor {u}.xyzw, {u}.xyzw, {h}.xyzw
utof {u}.xyzw, {u}.xyzw
mul {u}.xyzw, {u}.xyzw, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 2.328306437E-10)
add {u}.xyzw, {u}.xyzw, -{v}.xyzw
mad {v}.xyzw, {u}.xyzw, {p}.zzzz, {v}.xyzw
add {u}.xy, {v}.ywyy, -{v}.xzxx
mad {u}.xy, {u}.xyxx, {p}.xxxx, {v}.xzxx
add {u}.y, {u}.y, -{u}.x
mad {z}.w, {u}.y, {p}.y, {u}.x
dp3 {n}.x, {g}.xyzx, l(0.002040761, -0.001928961, 0.005168798, 0.000000)
dp3 {n}.y, {g}.xyzx, l(0.001718910, 0.005459080, 0.001358627, 0.000000)
dp3 {n}.z, {g}.xyzx, l(-0.005242396, 0.001039051, 0.002457586, 0.000000)
round_ni {o}.xyz, {n}.xyzx
add {n}.xyz, {n}.xyzx, -{o}.xyzx
mad {p}.xyz, {n}.xyzx, l(-2.000000, -2.000000, -2.000000, 0.000000), l(3.000000, 3.000000, 3.000000, 0.000000)
mul {r}.xyz, {n}.xyzx, {n}.xyzx
mul {p}.xyz, {p}.xyzx, {r}.xyzx
ftoi {o}.xyz, {o}.xyzx
iadd {s}.xyzw, {o}.xxxx, l(0, 1, 0, 1)
imul null, {s}.xyzw, {s}.xyzw, l(0x8da6b343, 0x8da6b343, 0x8da6b343, 0x8da6b343)
iadd {t}.xyzw, {o}.yyyy, l(0, 0, 1, 1)
imul null, {t}.xyzw, {t}.xyzw, l(0xd8163841, 0xd8163841, 0xd8163841, 0xd8163841)
xor {s}.xyzw, {s}.xyzw, {t}.xyzw
iadd {t}.xy, {o}.zzzz, l(0, 1, 0, 0)
imul null, {t}.xy, {t}.xyxx, l(0xcb1ab31f, 0xcb1ab31f, 0, 0)
xor {v}.xyzw, {s}.xyzw, {t}.xxxx
xor {u}.xyzw, {s}.xyzw, {t}.yyyy
iadd {v}.xyzw, {v}.xyzw, l(0x68e31da4, 0x68e31da4, 0x68e31da4, 0x68e31da4)
ushr {h}.xyzw, {v}.xyzw, l(16, 16, 16, 16)
xor {v}.xyzw, {v}.xyzw, {h}.xyzw
imul null, {v}.xyzw, {v}.xyzw, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0x7feb352d)
ushr {h}.xyzw, {v}.xyzw, l(15, 15, 15, 15)
xor {v}.xyzw, {v}.xyzw, {h}.xyzw
imul null, {v}.xyzw, {v}.xyzw, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0x846ca68b)
ushr {h}.xyzw, {v}.xyzw, l(16, 16, 16, 16)
xor {v}.xyzw, {v}.xyzw, {h}.xyzw
utof {v}.xyzw, {v}.xyzw
mul {v}.xyzw, {v}.xyzw, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 2.328306437E-10)
iadd {u}.xyzw, {u}.xyzw, l(0x68e31da4, 0x68e31da4, 0x68e31da4, 0x68e31da4)
ushr {h}.xyzw, {u}.xyzw, l(16, 16, 16, 16)
xor {u}.xyzw, {u}.xyzw, {h}.xyzw
imul null, {u}.xyzw, {u}.xyzw, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0x7feb352d)
ushr {h}.xyzw, {u}.xyzw, l(15, 15, 15, 15)
xor {u}.xyzw, {u}.xyzw, {h}.xyzw
imul null, {u}.xyzw, {u}.xyzw, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0x846ca68b)
ushr {h}.xyzw, {u}.xyzw, l(16, 16, 16, 16)
xor {u}.xyzw, {u}.xyzw, {h}.xyzw
utof {u}.xyzw, {u}.xyzw
mul {u}.xyzw, {u}.xyzw, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 2.328306437E-10)
add {u}.xyzw, {u}.xyzw, -{v}.xyzw
mad {v}.xyzw, {u}.xyzw, {p}.zzzz, {v}.xyzw
add {u}.xy, {v}.ywyy, -{v}.xzxx
mad {u}.xy, {u}.xyxx, {p}.xxxx, {v}.xzxx
add {u}.y, {u}.y, -{u}.x
mad {j}.x, {u}.y, {p}.y, {u}.x
add {j}.z, {j}.x, l(-0.500000)
dp3 {n}.x, {g}.xyzx, l(-0.008860883, -0.009282467, 0.015340161, 0.000000)
dp3 {n}.y, {g}.xyzx, l(0.015150750, 0.005274353, 0.011943031, 0.000000)
dp3 {n}.z, {g}.xyzx, l(-0.009588511, 0.016912037, 0.004695046, 0.000000)
mad {n}.xyz, {j}.zzzz, l(1.200000, -0.800000, 0.600000, 0.000000), {n}.xyzx
round_ni {o}.xyz, {n}.xyzx
add {n}.xyz, {n}.xyzx, -{o}.xyzx
mad {p}.xyz, {n}.xyzx, l(-2.000000, -2.000000, -2.000000, 0.000000), l(3.000000, 3.000000, 3.000000, 0.000000)
mul {r}.xyz, {n}.xyzx, {n}.xyzx
mul {p}.xyz, {p}.xyzx, {r}.xyzx
ftoi {o}.xyz, {o}.xyzx
iadd {s}.xyzw, {o}.xxxx, l(0, 1, 0, 1)
imul null, {s}.xyzw, {s}.xyzw, l(0x8da6b343, 0x8da6b343, 0x8da6b343, 0x8da6b343)
iadd {t}.xyzw, {o}.yyyy, l(0, 0, 1, 1)
imul null, {t}.xyzw, {t}.xyzw, l(0xd8163841, 0xd8163841, 0xd8163841, 0xd8163841)
xor {s}.xyzw, {s}.xyzw, {t}.xyzw
iadd {t}.xy, {o}.zzzz, l(0, 1, 0, 0)
imul null, {t}.xy, {t}.xyxx, l(0xcb1ab31f, 0xcb1ab31f, 0, 0)
xor {v}.xyzw, {s}.xyzw, {t}.xxxx
xor {u}.xyzw, {s}.xyzw, {t}.yyyy
iadd {v}.xyzw, {v}.xyzw, l(0xb5297a4d, 0xb5297a4d, 0xb5297a4d, 0xb5297a4d)
ushr {h}.xyzw, {v}.xyzw, l(16, 16, 16, 16)
xor {v}.xyzw, {v}.xyzw, {h}.xyzw
imul null, {v}.xyzw, {v}.xyzw, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0x7feb352d)
ushr {h}.xyzw, {v}.xyzw, l(15, 15, 15, 15)
xor {v}.xyzw, {v}.xyzw, {h}.xyzw
imul null, {v}.xyzw, {v}.xyzw, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0x846ca68b)
ushr {h}.xyzw, {v}.xyzw, l(16, 16, 16, 16)
xor {v}.xyzw, {v}.xyzw, {h}.xyzw
utof {v}.xyzw, {v}.xyzw
mul {v}.xyzw, {v}.xyzw, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 2.328306437E-10)
iadd {u}.xyzw, {u}.xyzw, l(0xb5297a4d, 0xb5297a4d, 0xb5297a4d, 0xb5297a4d)
ushr {h}.xyzw, {u}.xyzw, l(16, 16, 16, 16)
xor {u}.xyzw, {u}.xyzw, {h}.xyzw
imul null, {u}.xyzw, {u}.xyzw, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0x7feb352d)
ushr {h}.xyzw, {u}.xyzw, l(15, 15, 15, 15)
xor {u}.xyzw, {u}.xyzw, {h}.xyzw
imul null, {u}.xyzw, {u}.xyzw, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0x846ca68b)
ushr {h}.xyzw, {u}.xyzw, l(16, 16, 16, 16)
xor {u}.xyzw, {u}.xyzw, {h}.xyzw
utof {u}.xyzw, {u}.xyzw
mul {u}.xyzw, {u}.xyzw, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 2.328306437E-10)
add {u}.xyzw, {u}.xyzw, -{v}.xyzw
mad {v}.xyzw, {u}.xyzw, {p}.zzzz, {v}.xyzw
add {u}.xy, {v}.ywyy, -{v}.xzxx
mad {u}.xy, {u}.xyxx, {p}.xxxx, {v}.xzxx
add {u}.y, {u}.y, -{u}.x
mad {j}.y, {u}.y, {p}.y, {u}.x
add {z}.w, {z}.w, l(-0.500000)
add {j}.y, {j}.y, l(-0.500000)
mad {m}.x, {z}.w, l(1.200000), {m}.x
mad {m}.x, {j}.z, l(1.800000), {m}.x
mad {m}.x, {j}.y, l(1.400000), {m}.x
mul {d}.xyz, cb2[12].xyzx, l(-0.0009765625, -0.0009765625, -0.0009765625, 0.000000)
round_ne {d}.xyz, {d}.xyzx
mul {d}.xyz, {d}.xyzx, l(1024.000000, 1024.000000, 1024.000000, 0.000000)
add {w}.xyz, -cb2[12].xyzx, -{d}.xyzx
add {w}.xyz, {w}.xyzx, v3.xyzx
dp3 {n}.x, {w}.xyzx, l(0.081791528, -0.008318472, 0.116829415, 0.000000)
dp3 {n}.y, {w}.xyzx, l(0.103070266, 0.072793734, -0.066975786, 0.000000)
dp3 {n}.z, {w}.xyzx, l(-0.009735459, 0.021461621, 0.008343840, 0.000000)
dp3 {o}.x, {d}.xyzx, l(0.081791528, -0.008318472, 0.116829415, 0.000000)
dp3 {o}.y, {d}.xyzx, l(0.103070266, 0.072793734, -0.066975786, 0.000000)
dp3 {o}.z, {d}.xyzx, l(-0.009735459, 0.021461621, 0.008343840, 0.000000)
round_ni {p}.xyz, {o}.xyzx
add {o}.xyz, {o}.xyzx, -{p}.xyzx
add {n}.xyz, {n}.xyzx, {o}.xyzx
round_ni {o}.xyz, {n}.xyzx
add {n}.xyz, {n}.xyzx, -{o}.xyzx
add {o}.xyz, {o}.xyzx, {p}.xyzx
mad {p}.xyz, {n}.xyzx, l(-2.000000, -2.000000, -2.000000, 0.000000), l(3.000000, 3.000000, 3.000000, 0.000000)
mul {r}.xyz, {n}.xyzx, {n}.xyzx
mul {p}.xyz, {p}.xyzx, {r}.xyzx
ftoi {o}.xyz, {o}.xyzx
iadd {s}.xyzw, {o}.xxxx, l(0, 1, 0, 1)
imul null, {s}.xyzw, {s}.xyzw, l(0x8da6b343, 0x8da6b343, 0x8da6b343, 0x8da6b343)
iadd {t}.xyzw, {o}.yyyy, l(0, 0, 1, 1)
imul null, {t}.xyzw, {t}.xyzw, l(0xd8163841, 0xd8163841, 0xd8163841, 0xd8163841)
xor {s}.xyzw, {s}.xyzw, {t}.xyzw
iadd {t}.xy, {o}.zzzz, l(0, 1, 0, 0)
imul null, {t}.xy, {t}.xyxx, l(0xcb1ab31f, 0xcb1ab31f, 0, 0)
xor {v}.xyzw, {s}.xyzw, {t}.xxxx
xor {u}.xyzw, {s}.xyzw, {t}.yyyy
iadd {v}.xyzw, {v}.xyzw, l(0x2c1b3c6d, 0x2c1b3c6d, 0x2c1b3c6d, 0x2c1b3c6d)
ushr {h}.xyzw, {v}.xyzw, l(16, 16, 16, 16)
xor {v}.xyzw, {v}.xyzw, {h}.xyzw
imul null, {v}.xyzw, {v}.xyzw, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0x7feb352d)
ushr {h}.xyzw, {v}.xyzw, l(15, 15, 15, 15)
xor {v}.xyzw, {v}.xyzw, {h}.xyzw
imul null, {v}.xyzw, {v}.xyzw, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0x846ca68b)
ushr {h}.xyzw, {v}.xyzw, l(16, 16, 16, 16)
xor {v}.xyzw, {v}.xyzw, {h}.xyzw
utof {v}.xyzw, {v}.xyzw
mul {v}.xyzw, {v}.xyzw, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 2.328306437E-10)
iadd {u}.xyzw, {u}.xyzw, l(0x2c1b3c6d, 0x2c1b3c6d, 0x2c1b3c6d, 0x2c1b3c6d)
ushr {h}.xyzw, {u}.xyzw, l(16, 16, 16, 16)
xor {u}.xyzw, {u}.xyzw, {h}.xyzw
imul null, {u}.xyzw, {u}.xyzw, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0x7feb352d)
ushr {h}.xyzw, {u}.xyzw, l(15, 15, 15, 15)
xor {u}.xyzw, {u}.xyzw, {h}.xyzw
imul null, {u}.xyzw, {u}.xyzw, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0x846ca68b)
ushr {h}.xyzw, {u}.xyzw, l(16, 16, 16, 16)
xor {u}.xyzw, {u}.xyzw, {h}.xyzw
utof {u}.xyzw, {u}.xyzw
mul {u}.xyzw, {u}.xyzw, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 2.328306437E-10)
add {u}.xyzw, {u}.xyzw, -{v}.xyzw
mad {v}.xyzw, {u}.xyzw, {p}.zzzz, {v}.xyzw
add {u}.xy, {v}.ywyy, -{v}.xzxx
mad {u}.xy, {u}.xyxx, {p}.xxxx, {v}.xzxx
add {u}.y, {u}.y, -{u}.x
mad {j}.x, {u}.y, {p}.y, {u}.x
dp3 {n}.x, {w}.xyzx, l(0.229016278, -0.023291722, 0.327122362, 0.000000)
dp3 {n}.y, {w}.xyzx, l(0.288596745, 0.203822454, -0.187532200, 0.000000)
dp3 {n}.z, {w}.xyzx, l(-0.027815596, 0.061318918, 0.023839542, 0.000000)
dp3 {o}.x, {d}.xyzx, l(0.229016278, -0.023291722, 0.327122362, 0.000000)
dp3 {o}.y, {d}.xyzx, l(0.288596745, 0.203822454, -0.187532200, 0.000000)
dp3 {o}.z, {d}.xyzx, l(-0.027815596, 0.061318918, 0.023839542, 0.000000)
round_ni {p}.xyz, {o}.xyzx
add {o}.xyz, {o}.xyzx, -{p}.xyzx
add {n}.xyz, {n}.xyzx, {o}.xyzx
round_ni {o}.xyz, {n}.xyzx
add {n}.xyz, {n}.xyzx, -{o}.xyzx
add {o}.xyz, {o}.xyzx, {p}.xyzx
mad {p}.xyz, {n}.xyzx, l(-2.000000, -2.000000, -2.000000, 0.000000), l(3.000000, 3.000000, 3.000000, 0.000000)
mul {r}.xyz, {n}.xyzx, {n}.xyzx
mul {p}.xyz, {p}.xyzx, {r}.xyzx
ftoi {o}.xyz, {o}.xyzx
iadd {s}.xyzw, {o}.xxxx, l(0, 1, 0, 1)
imul null, {s}.xyzw, {s}.xyzw, l(0x8da6b343, 0x8da6b343, 0x8da6b343, 0x8da6b343)
iadd {t}.xyzw, {o}.yyyy, l(0, 0, 1, 1)
imul null, {t}.xyzw, {t}.xyzw, l(0xd8163841, 0xd8163841, 0xd8163841, 0xd8163841)
xor {s}.xyzw, {s}.xyzw, {t}.xyzw
iadd {t}.xy, {o}.zzzz, l(0, 1, 0, 0)
imul null, {t}.xy, {t}.xyxx, l(0xcb1ab31f, 0xcb1ab31f, 0, 0)
xor {v}.xyzw, {s}.xyzw, {t}.xxxx
xor {u}.xyzw, {s}.xyzw, {t}.yyyy
iadd {v}.xyzw, {v}.xyzw, l(0x297a2d39, 0x297a2d39, 0x297a2d39, 0x297a2d39)
ushr {h}.xyzw, {v}.xyzw, l(16, 16, 16, 16)
xor {v}.xyzw, {v}.xyzw, {h}.xyzw
imul null, {v}.xyzw, {v}.xyzw, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0x7feb352d)
ushr {h}.xyzw, {v}.xyzw, l(15, 15, 15, 15)
xor {v}.xyzw, {v}.xyzw, {h}.xyzw
imul null, {v}.xyzw, {v}.xyzw, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0x846ca68b)
ushr {h}.xyzw, {v}.xyzw, l(16, 16, 16, 16)
xor {v}.xyzw, {v}.xyzw, {h}.xyzw
utof {v}.xyzw, {v}.xyzw
mul {v}.xyzw, {v}.xyzw, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 2.328306437E-10)
iadd {u}.xyzw, {u}.xyzw, l(0x297a2d39, 0x297a2d39, 0x297a2d39, 0x297a2d39)
ushr {h}.xyzw, {u}.xyzw, l(16, 16, 16, 16)
xor {u}.xyzw, {u}.xyzw, {h}.xyzw
imul null, {u}.xyzw, {u}.xyzw, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0x7feb352d)
ushr {h}.xyzw, {u}.xyzw, l(15, 15, 15, 15)
xor {u}.xyzw, {u}.xyzw, {h}.xyzw
imul null, {u}.xyzw, {u}.xyzw, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0x846ca68b)
ushr {h}.xyzw, {u}.xyzw, l(16, 16, 16, 16)
xor {u}.xyzw, {u}.xyzw, {h}.xyzw
utof {u}.xyzw, {u}.xyzw
mul {u}.xyzw, {u}.xyzw, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 2.328306437E-10)
add {u}.xyzw, {u}.xyzw, -{v}.xyzw
mad {v}.xyzw, {u}.xyzw, {p}.zzzz, {v}.xyzw
add {u}.xy, {v}.ywyy, -{v}.xzxx
mad {u}.xy, {u}.xyxx, {p}.xxxx, {v}.xzxx
add {u}.y, {u}.y, -{u}.x
mad {j}.y, {u}.y, {p}.y, {u}.x
add {j}.xy, {j}.xyxx, l(-0.500000, -0.500000, 0.000000, 0.000000)
mul {j}.x, {j}.x, l(2.800000)
mad {j}.x, {j}.y, l(2.800000), {j}.x
mad {m}.x, {j}.x, {m}.y, {m}.x
else
mov {z}.w, l(0.000000)
endif
// EDCAS parallax occlusion mapping: z.xy the uv shift, z.z the shadow
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(514, 0, 0, 0), t120.xyzw
eq {h}.x, {j}.z, l(0.000000)
movc {g}.z, {h}.x, l(1.000000), {j}.z
ge {h}.x, {j}.x, l(1.500000)
lt {h}.y, {j}.x, l(3.500000)
and {h}.x, {h}.x, {h}.y
ge {h}.y, {j}.z, l(0.000000)
and {e}.w, {h}.x, {h}.y
max {g}.z, {g}.z, l(0.000000)
if_nz {e}.w
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(517, 0, 0, 0), t120.xyzw
eq {h}.x, {j}.x, l(0.000000)
movc {j}.x, {h}.x, l(0.500000), {j}.x
lt {h}.x, {j}.x, l(0.000000)
movc {j}.x, {h}.x, l(-10.000000), {j}.x
max {f}.x, {j}.x, l(0.020000)
min {f}.x, {f}.x, l(0.980000)
add {f}.y, -{f}.x, l(1.000000)
log {f}.x, {f}.x
log {f}.y, {f}.y
add {f}.x, {f}.x, -{f}.y
mad {f}.y, {f}.x, l(-0.002437424), l(-0.018739033)
mad {f}.y, {f}.y, {f}.x, l(-0.487268350)
mad {f}.y, {f}.y, {f}.x, l(0.264974522)
add {f}.x, {m}.x, -{f}.y
mad_sat {g}.w, {f}.x, l(0.400000), l(0.500000)
deriv_rtx_fine {n}.xyz, v3.xyzx
deriv_rty_fine {o}.xyz, v3.xyzx
deriv_rtx_fine {p}.xy, v1.xyxx
deriv_rty_fine {p}.zw, v1.xxxy
dp3 {r}.x, {n}.xyzx, {n}.xyzx
dp3 {r}.y, {o}.xyzx, {o}.xyzx
dp3 {r}.z, {n}.xyzx, {o}.xyzx
mul {t}.xyz, {n}.yzxx, {o}.zxyx
mad {t}.xyz, -{n}.zxyx, {o}.yzxx, {t}.xyzx
dp3 {u}.x, v3.xyzx, {t}.xyzx
dp3 {u}.y, {t}.xyzx, {t}.xyzx
dp3 {u}.z, v3.xyzx, v3.xyzx
dp3 {s}.x, v3.xyzx, {n}.xyzx
dp3 {s}.y, v3.xyzx, {o}.xyzx
max {w}.x, {r}.x, {r}.y
sqrt {w}.x, {w}.x
lt {w}.y, l(0.000000), {c}.w
movc {w}.y, {w}.y, {c}.w, l(1.000000)
div {w}.x, {w}.x, {w}.y
mad_sat {w}.y, {w}.x, l(-50.000000), l(2.000000)
mul {w}.y, {w}.y, {g}.z
mad_sat {w}.z, {g}.w, l(10.000000), l(-0.500000)
mul {w}.y, {w}.y, {w}.z
mov {e}.x, {w}.y
ld_indexable(texture1d)(float,float,float,float) {h}.xyzw, l(514, 0, 0, 0), t120.xyzw
eq {h}.x, {h}.w, l(0.000000)
movc {e}.y, {h}.x, l(1.000000), {h}.w
max {e}.y, {e}.y, l(0.000000)
mul {e}.y, {e}.y, l(0.025000)
mul {w}.x, {w}.x, l(512.000000)
max {w}.x, {w}.x, l(1.000000)
log {e}.z, {w}.x
lt {h}.w, l(0.000000), {e}.x
if_nz {h}.w
// EDCAS parallax: only where its height has a weight
mul {w}.x, {u}.x, {u}.x
mul {w}.y, {u}.z, {u}.y
div {w}.y, {w}.y, {w}.x
add {w}.y, {w}.y, l(-1.000000)
max {w}.y, {w}.y, l(0.000000)
min {k}.y, {w}.y, l(16.000000)
rsq {k}.y, {k}.y
mul {k}.y, {k}.y, l(0.006000)
min {k}.y, {k}.y, l(0.002113)
rsq {w}.y, {w}.y
mul {w}.y, {w}.y, l(4.000000)
min {w}.y, {w}.y, l(1.000000)
mul {w}.z, {s}.x, {r}.y
mad {w}.z, -{s}.y, {r}.z, {w}.z
mul {w}.w, {s}.y, {r}.x
mad {w}.w, -{s}.x, {r}.z, {w}.w
mul {u}.w, {r}.x, {r}.y
mad {u}.w, -{r}.z, {r}.z, {u}.w
sqrt {w}.x, {u}.y
div {w}.x, {w}.x, |{u}.x|
div {w}.x, {w}.x, {u}.w
mul {w}.x, {w}.x, {w}.y
mul {f}.x, {w}.z, {p}.x
mad {f}.x, {w}.w, {p}.z, {f}.x
mul {f}.y, {w}.z, {p}.y
mad {f}.y, {w}.w, {p}.w, {f}.y
mul {f}.xy, {f}.xyxx, -{w}.xxxx
dp3 {w}.x, cb1[4].xyzx, {n}.xyzx
dp3 {w}.y, cb1[4].xyzx, {o}.xyzx
dp3 {w}.z, cb1[4].xyzx, {t}.xyzx
lt {j}.w, {u}.x, l(0.000000)
movc {w}.z, {j}.w, -{w}.z, {w}.z
sqrt {t}.x, {u}.y
mul {t}.y, {t}.x, l(0.001000)
lt {j}.w, {t}.y, {w}.z
and {j}.w, {j}.w, {e}.w
mul {w}.w, {w}.z, {w}.z
div {w}.w, {u}.y, {w}.w
add {w}.w, {w}.w, l(-1.000000)
max {w}.w, {w}.w, l(0.000000)
rsq {w}.w, {w}.w
mul {w}.w, {w}.w, l(4.000000)
min {w}.w, {w}.w, l(1.000000)
mul {s}.z, {w}.x, {r}.y
mad {s}.z, -{w}.y, {r}.z, {s}.z
mul {s}.w, {w}.y, {r}.x
mad {s}.w, -{w}.x, {r}.z, {s}.w
sqrt {t}.x, {u}.y
div {t}.x, {t}.x, {w}.z
div {t}.x, {t}.x, {u}.w
mul {t}.x, {t}.x, {w}.w
mul {g}.x, {s}.z, {p}.x
mad {g}.x, {s}.w, {p}.z, {g}.x
mul {g}.y, {s}.z, {p}.y
mad {g}.y, {s}.w, {p}.w, {g}.y
mul {g}.xy, {g}.xyxx, -{t}.xxxx
mul {k}.w, {a}.y, l(0.250000)
round_ne {k}.w, {k}.w
max {k}.w, {k}.w, l(1.000000)
mul {k}.w, {k}.w, {a}.x
mul {d}.xy, {f}.xyxx, {k}.wwww
mul {d}.zw, {g}.xxxy, {k}.wwww
mul {a}.zw, v1.xxxy, {k}.wwww
div {w}.x, {k}.w, {a}.x
mul {w}.y, {w}.x, l(0.57735027)
round_ne {w}.y, {w}.y
max {w}.y, {w}.y, l(1.000000)
add {w}.y, {w}.y, {w}.y
mul {w}.z, {w}.y, {a}.x
div {w}.w, {w}.z, {k}.w
mul {g}.y, {a}.w, {w}.w
mad {g}.x, {g}.y, l(-0.500000), {a}.z
round_ni {h}.xy, {g}.xyxx
add {g}.xy, {g}.xyxx, -{h}.xyxx
add {g}.z, {g}.x, {g}.y
lt {f}.w, l(1.000000), {g}.z
and {f}.w, {f}.w, l(1065353216)
mad {w}.w, {f}.w, l(2.000000), l(-1.000000)
add {g}.z, {g}.z, l(-1.000000)
mul {u}.x, {w}.w, {g}.z
mad {u}.y, -{w}.w, {g}.x, {f}.w
mad {u}.z, -{w}.w, {g}.y, {f}.w
add {w}.w, -{f}.w, l(1.000000)
add {n}.x, {h}.x, {f}.w
add {n}.y, {h}.x, {w}.w
mov {n}.z, {n}.x
add {o}.x, {h}.y, {f}.w
mov {o}.y, {o}.x
add {o}.z, {h}.y, {w}.w
div {f}.xyz, {o}.xyzx, {w}.yyyy
round_ni {f}.xyz, {f}.xyzx
mad {g}.xyz, -{f}.xyzx, {w}.yyyy, {o}.xyzx
mul {h}.xyz, {f}.xyzx, {w}.yyyy
mad {h}.xyz, {h}.xyzx, l(0.500000), {n}.xyzx
div {f}.xyz, {h}.xyzx, {w}.xxxx
round_ni {f}.xyz, {f}.xyzx
mad {h}.xyz, -{f}.xyzx, {w}.xxxx, {h}.xyzx
mad {p}.xyz, {h}.xyzx, {w}.yyyy, {g}.xyzx
ftoi {p}.xyz, {p}.xyzx
iadd {f}.xyz, {p}.xyzx, l(0x00000000, 0x00000000, 0x00000000, 0)
ushr {h}.xyz, {f}.xyzx, l(16, 16, 16, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
imul null, {f}.xyz, {f}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {f}.xyzx, l(15, 15, 15, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
imul null, {f}.xyz, {f}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {f}.xyzx, l(16, 16, 16, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
utof {f}.xyz, {f}.xyzx
mul {f}.xyz, {f}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
iadd {g}.xyz, {p}.xyzx, l(0x68e31da4, 0x68e31da4, 0x68e31da4, 0)
ushr {h}.xyz, {g}.xyzx, l(16, 16, 16, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
imul null, {g}.xyz, {g}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {g}.xyzx, l(15, 15, 15, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
imul null, {g}.xyz, {g}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {g}.xyzx, l(16, 16, 16, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
utof {g}.xyz, {g}.xyzx
mul {g}.xyz, {g}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
iadd {t}.xyz, {p}.xyzx, l(0xb5297a4d, 0xb5297a4d, 0xb5297a4d, 0)
ushr {h}.xyz, {t}.xyzx, l(16, 16, 16, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
imul null, {t}.xyz, {t}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {t}.xyzx, l(15, 15, 15, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
imul null, {t}.xyz, {t}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {t}.xyzx, l(16, 16, 16, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
utof {t}.xyz, {t}.xyzx
mul {t}.xyz, {t}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
div {w}.w, {k}.w, {w}.z
mad {n}.xyz, {o}.xyzx, l(0.500000), {n}.xyzx
mul {o}.xyz, {o}.xyzx, {w}.wwww
add {p}.xyz, {n}.xyzx, {f}.xyzx
add {r}.xyz, {o}.xyzx, {g}.xyzx
mul {h}.xyz, {t}.xyzx, l(6.28318531, 6.28318531, 6.28318531, 0.000000)
mul {g}.xyz, {t}.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000)
round_ni {g}.xyz, {g}.xyzx
mul {g}.xyz, {g}.xyzx, l(3.14159265, 3.14159265, 3.14159265, 0.000000)
eq {w}.x, {q}.z, l(0.000000)
movc {h}.xyz, {w}.xxxx, {g}.xyzx, {h}.xyzx
sincos {t}.x, {s}.x, {h}.x
sincos {t}.y, {s}.y, {h}.y
sincos {t}.z, {s}.z, {h}.z
mul {v}.xyz, {u}.xyzx, {u}.xyzx
mul {v}.xyz, {v}.xyzx, {v}.xyzx
dp3 {w}.x, {v}.xyzx, l(1.000000, 1.000000, 1.000000, 0.000000)
div {v}.xyz, {v}.xyzx, {w}.xxxx
lt {w}.x, {m}.w, l(0.000000)
movc {s}.xyz, {w}.xxxx, l(1.000000, 1.000000, 1.000000, 0.000000), {s}.xyzx
movc {t}.xyz, {w}.xxxx, l(0.000000, 0.000000, 0.000000, 0.000000), {t}.xyzx
movc {p}.xyz, {w}.xxxx, {n}.xyzx, {p}.xyzx
movc {r}.xyz, {w}.xxxx, {o}.xyzx, {r}.xyzx
movc {u}.xyz, {w}.xxxx, l(1.000000, 0.000000, 0.000000, 0.000000), {u}.xyzx
movc {v}.xyz, {w}.xxxx, l(1.000000, 0.000000, 0.000000, 0.000000), {v}.xyzx
add {h}.xyz, {a}.zzzz, -{n}.xyzx
add {f}.xyz, {a}.wwww, -{o}.xyzx
mul {n}.xyz, {s}.xyzx, {h}.xyzx
mad {n}.xyz, -{t}.xyzx, {f}.xyzx, {n}.xyzx
add {n}.xyz, {n}.xyzx, {p}.xyzx
mul {o}.xyz, {t}.xyzx, {h}.xyzx
mad {o}.xyz, {s}.xyzx, {f}.xyzx, {o}.xyzx
add {o}.xyz, {o}.xyzx, {r}.xyzx
eq {p}.x, l(0.000000), l(0.000000)
mov {p}.y, l(-1.000000)
mad {p}.z, {k}.y, l(-71.000000), l(0.070000)
mov {w}.y, {k}.y
mov {w}.z, l(0.070000)
mul {r}.xy, {d}.xyxx, l(0.070000)
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, l(-0.070000)
ge {w}.x, {u}.x, l(0.000000)
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, l(0.070000), {p}.z
xor {p}.x, {p}.x, {u}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
if_nz {p}.x
// EDCAS parallax: the next steps, while no hit is found
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
endif
if_nz {p}.x
// EDCAS parallax: the next steps, while no hit is found
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
endif
if_nz {p}.x
// EDCAS parallax: the next steps, while no hit is found
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
endif
if_nz {p}.x
// EDCAS parallax: the next steps, while no hit is found
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
endif
if_nz {p}.x
// EDCAS parallax: the next steps, while no hit is found
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
endif
if_nz {p}.x
// EDCAS parallax: the next steps, while no hit is found
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
endif
if_nz {p}.x
// EDCAS parallax: the next steps, while no hit is found
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
endif
if_nz {p}.x
// EDCAS parallax: the next steps, while no hit is found
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
add {w}.w, {w}.z, -{w}.y
mul {r}.xy, {d}.xyxx, {w}.wwww
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
add {u}.x, {u}.x, -{w}.w
ge {w}.x, {u}.x, l(0.000000)
add {u}.y, {p}.y, -{u}.x
div {u}.y, {p}.y, {u}.y
mad {u}.z, -{u}.y, {w}.y, {w}.z
and {u}.w, {w}.x, {p}.x
movc {p}.z, {u}.w, {u}.z, {p}.z
xor {p}.x, {p}.x, {u}.w
mov {w}.z, {w}.w
mov {p}.y, {u}.x
endif
mov {k}.y, {p}.z
mul {z}.xy, {d}.xyxx, {k}.yyyy
div {z}.xy, {z}.xyxx, {k}.wwww
eq {h}.x, {z}.x, {z}.x
eq {h}.y, {z}.y, {z}.y
and {h}.x, {h}.x, {h}.y
and {h}.x, {h}.x, {e}.w
movc {z}.xy, {h}.xxxx, {z}.xyxx, l(0.000000, 0.000000, 0.000000, 0.000000)
mul {j}.xy, {d}.xyxx, {k}.yyyy
add {p}.w, l(0.070000), -{k}.y
mul {p}.xy, {d}.zwzz, {p}.wwww
mov {j}.z, l(0.000000)
mad {r}.xy, {p}.xyxx, l(0.125000), {j}.xyxx
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
mad {u}.y, {k}.y, l(0.875000), l(0.008750)
add {u}.x, {u}.x, -{u}.y
mad_sat {u}.x, {u}.x, l(200.000000), l(0.000000)
add {j}.z, {j}.z, {u}.x
mad {r}.xy, {p}.xyxx, l(0.250000), {j}.xyxx
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
mad {u}.y, {k}.y, l(0.750000), l(0.017500)
add {u}.x, {u}.x, -{u}.y
mad_sat {u}.x, {u}.x, l(200.000000), l(0.000000)
add {j}.z, {j}.z, {u}.x
mad {r}.xy, {p}.xyxx, l(0.375000), {j}.xyxx
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
mad {u}.y, {k}.y, l(0.625000), l(0.026250)
add {u}.x, {u}.x, -{u}.y
mad_sat {u}.x, {u}.x, l(200.000000), l(0.000000)
add {j}.z, {j}.z, {u}.x
mad {r}.xy, {p}.xyxx, l(0.500000), {j}.xyxx
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
mad {u}.y, {k}.y, l(0.500000), l(0.035000)
add {u}.x, {u}.x, -{u}.y
mad_sat {u}.x, {u}.x, l(200.000000), l(0.000000)
add {j}.z, {j}.z, {u}.x
mad {r}.xy, {p}.xyxx, l(0.625000), {j}.xyxx
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
mad {u}.y, {k}.y, l(0.375000), l(0.043750)
add {u}.x, {u}.x, -{u}.y
mad_sat {u}.x, {u}.x, l(200.000000), l(0.000000)
add {j}.z, {j}.z, {u}.x
mad {r}.xy, {p}.xyxx, l(0.750000), {j}.xyxx
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
mad {u}.y, {k}.y, l(0.250000), l(0.052500)
add {u}.x, {u}.x, -{u}.y
mad_sat {u}.x, {u}.x, l(200.000000), l(0.000000)
add {j}.z, {j}.z, {u}.x
mad {r}.xy, {p}.xyxx, l(0.875000), {j}.xyxx
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
mad {u}.y, {k}.y, l(0.125000), l(0.061250)
add {u}.x, {u}.x, -{u}.y
mad_sat {u}.x, {u}.x, l(200.000000), l(0.000000)
add {j}.z, {j}.z, {u}.x
mad {r}.xy, {p}.xyxx, l(1.000000), {j}.xyxx
mad {f}.xyz, {s}.xyzx, {r}.xxxx, {n}.xyzx
mad {f}.xyz, -{t}.xyzx, {r}.yyyy, {f}.xyzx
mad {g}.xyz, {t}.xyzx, {r}.xxxx, {o}.xyzx
mad {g}.xyz, {s}.xyzx, {r}.yyyy, {g}.xyzx
frc {f}.xyz, {f}.xyzx
frc {g}.xyz, {g}.xyzx
mov {q}.x, {f}.x
mov {q}.y, {g}.x
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mul {u}.xy, {h}.xyxx, {v}.xxxx
mov {q}.x, {f}.y
mov {q}.y, {g}.y
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.yyyy, {u}.xyxx
mov {q}.x, {f}.z
mov {q}.y, {g}.z
sample_l_indexable(texture2darray)(float,float,float,float) {h}.xy, {q}.xyzx, t117.xyzw, s0, {e}.z
mad {u}.xy, {h}.xyxx, {v}.zzzz, {u}.xyxx
mad {u}.xy, {u}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)
mad_sat {u}.z, {u}.x, l(32.786885), l(0.377049)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
add {u}.z, {u}.z, {g}.w
mad_sat {u}.z, {u}.z, l(6.000000), l(-5.400000)
mad {u}.w, {u}.z, l(-2.000000), l(3.000000)
mul {u}.z, {u}.z, {u}.z
mul {u}.z, {u}.z, {u}.w
mad_sat {u}.w, {g}.w, l(3.333333), l(-2.333333)
mul {u}.x, {u}.x, {u}.w
add {u}.y, {u}.y, {e}.y
mad {u}.x, {u}.z, {u}.y, {u}.x
mul {u}.x, {u}.x, {e}.x
mad {u}.y, {k}.y, l(0.000000), l(0.070000)
add {u}.x, {u}.x, -{u}.y
mad_sat {u}.x, {u}.x, l(200.000000), l(0.000000)
add {j}.z, {j}.z, {u}.x
mul {j}.z, {j}.z, l(0.125000)
movc {j}.z, {j}.w, {j}.z, l(0.000000)
add {u}.x, l(0.070000), -{k}.y
mul {u}.x, {u}.x, l(6.666667)
eq {u}.y, {c}.y, l(10.000000)
movc {z}.z, {u}.y, {u}.x, {j}.z
else
mov {z}.xyz, l(0.000000, 0.000000, 0.000000, 0.000000)
endif
else
mov {z}.xyz, l(0.000000, 0.000000, 0.000000, 0.000000)
endif
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(514, 0, 0, 0), t120.xyzw
mov {q}.z, {j}.x
mul {a}.zw, v1.xxxy, {b}.xxxx
deriv_rtx_fine {d}.xy, {a}.zwzz
deriv_rty_fine {d}.zw, {a}.zzzw
mad {a}.zw, {z}.xxxy, {b}.xxxx, {a}.xyzw
frc {q}.xy, {a}.zwzz
sample_d_indexable(texture2darray)(float,float,float,float) {e}.x, {q}.xyzx, t110.xyzw, s0, {d}.xyxx, {d}.zwzz
mul {a}.zw, v1.xxxy, {b}.yyyy
deriv_rtx_fine {d}.xy, {a}.zwzz
deriv_rty_fine {d}.zw, {a}.zzzw
mad {a}.zw, {z}.xxxy, {b}.yyyy, {a}.xyzw
div {w}.x, {b}.y, {a}.x
mul {w}.y, {w}.x, l(0.57735027)
round_ne {w}.y, {w}.y
max {w}.y, {w}.y, l(1.000000)
add {w}.y, {w}.y, {w}.y
mul {w}.z, {w}.y, {a}.x
div {w}.w, {w}.z, {b}.y
mul {g}.y, {a}.w, {w}.w
mad {g}.x, {g}.y, l(-0.500000), {a}.z
round_ni {h}.xy, {g}.xyxx
add {g}.xy, {g}.xyxx, -{h}.xyxx
add {g}.z, {g}.x, {g}.y
lt {f}.w, l(1.000000), {g}.z
and {f}.w, {f}.w, l(1065353216)
mad {w}.w, {f}.w, l(2.000000), l(-1.000000)
add {g}.z, {g}.z, l(-1.000000)
mul {u}.x, {w}.w, {g}.z
mad {u}.y, -{w}.w, {g}.x, {f}.w
mad {u}.z, -{w}.w, {g}.y, {f}.w
add {w}.w, -{f}.w, l(1.000000)
add {n}.x, {h}.x, {f}.w
add {n}.y, {h}.x, {w}.w
mov {n}.z, {n}.x
add {o}.x, {h}.y, {f}.w
mov {o}.y, {o}.x
add {o}.z, {h}.y, {w}.w
div {f}.xyz, {o}.xyzx, {w}.yyyy
round_ni {f}.xyz, {f}.xyzx
mad {g}.xyz, -{f}.xyzx, {w}.yyyy, {o}.xyzx
mul {h}.xyz, {f}.xyzx, {w}.yyyy
mad {h}.xyz, {h}.xyzx, l(0.500000), {n}.xyzx
div {f}.xyz, {h}.xyzx, {w}.xxxx
round_ni {f}.xyz, {f}.xyzx
mad {h}.xyz, -{f}.xyzx, {w}.xxxx, {h}.xyzx
mad {p}.xyz, {h}.xyzx, {w}.yyyy, {g}.xyzx
ftoi {p}.xyz, {p}.xyzx
iadd {f}.xyz, {p}.xyzx, l(0x00000000, 0x00000000, 0x00000000, 0)
ushr {h}.xyz, {f}.xyzx, l(16, 16, 16, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
imul null, {f}.xyz, {f}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {f}.xyzx, l(15, 15, 15, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
imul null, {f}.xyz, {f}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {f}.xyzx, l(16, 16, 16, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
utof {f}.xyz, {f}.xyzx
mul {f}.xyz, {f}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
iadd {g}.xyz, {p}.xyzx, l(0x68e31da4, 0x68e31da4, 0x68e31da4, 0)
ushr {h}.xyz, {g}.xyzx, l(16, 16, 16, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
imul null, {g}.xyz, {g}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {g}.xyzx, l(15, 15, 15, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
imul null, {g}.xyz, {g}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {g}.xyzx, l(16, 16, 16, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
utof {g}.xyz, {g}.xyzx
mul {g}.xyz, {g}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
iadd {t}.xyz, {p}.xyzx, l(0xb5297a4d, 0xb5297a4d, 0xb5297a4d, 0)
ushr {h}.xyz, {t}.xyzx, l(16, 16, 16, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
imul null, {t}.xyz, {t}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {t}.xyzx, l(15, 15, 15, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
imul null, {t}.xyz, {t}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {t}.xyzx, l(16, 16, 16, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
utof {t}.xyz, {t}.xyzx
mul {t}.xyz, {t}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
div {w}.w, {b}.y, {w}.z
mad {n}.xyz, {o}.xyzx, l(0.500000), {n}.xyzx
mul {o}.xyz, {o}.xyzx, {w}.wwww
add {p}.xyz, {n}.xyzx, {f}.xyzx
add {r}.xyz, {o}.xyzx, {g}.xyzx
mul {h}.xyz, {t}.xyzx, l(6.28318531, 6.28318531, 6.28318531, 0.000000)
mul {g}.xyz, {t}.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000)
round_ni {g}.xyz, {g}.xyzx
mul {g}.xyz, {g}.xyzx, l(3.14159265, 3.14159265, 3.14159265, 0.000000)
eq {w}.x, {q}.z, l(0.000000)
movc {h}.xyz, {w}.xxxx, {g}.xyzx, {h}.xyzx
sincos {t}.x, {s}.x, {h}.x
sincos {t}.y, {s}.y, {h}.y
sincos {t}.z, {s}.z, {h}.z
mul {v}.xyz, {u}.xyzx, {u}.xyzx
mul {v}.xyz, {v}.xyzx, {v}.xyzx
dp3 {w}.x, {v}.xyzx, l(1.000000, 1.000000, 1.000000, 0.000000)
div {v}.xyz, {v}.xyzx, {w}.xxxx
lt {w}.x, {m}.w, l(0.000000)
movc {s}.xyz, {w}.xxxx, l(1.000000, 1.000000, 1.000000, 0.000000), {s}.xyzx
movc {t}.xyz, {w}.xxxx, l(0.000000, 0.000000, 0.000000, 0.000000), {t}.xyzx
movc {p}.xyz, {w}.xxxx, {n}.xyzx, {p}.xyzx
movc {r}.xyz, {w}.xxxx, {o}.xyzx, {r}.xyzx
movc {u}.xyz, {w}.xxxx, l(1.000000, 0.000000, 0.000000, 0.000000), {u}.xyzx
movc {v}.xyz, {w}.xxxx, l(1.000000, 0.000000, 0.000000, 0.000000), {v}.xyzx
mov {e}.z, l(0.000000)
mov {j}.xy, l(0.000000, 0.000000, 0.000000, 0.000000)
add {h}.x, {a}.z, -{n}.x
add {h}.y, {a}.w, -{o}.x
mul {h}.z, {s}.x, {h}.x
mad {h}.z, -{t}.x, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.x
mul {h}.w, {t}.x, {h}.x
mad {h}.w, {s}.x, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.x
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.x, {d}.x
mad {f}.x, -{t}.x, {d}.y, {f}.x
mul {f}.y, {t}.x, {d}.x
mad {f}.y, {s}.x, {d}.y, {f}.y
mul {f}.z, {s}.x, {d}.z
mad {f}.z, -{t}.x, {d}.w, {f}.z
mul {f}.w, {t}.x, {d}.z
mad {f}.w, {s}.x, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.z, {q}.xyzx, t113.xyxy, s0, {f}.xyxx, {f}.zwzz
mad {e}.z, {g}.z, {v}.x, {e}.z
sample_d_indexable(texture2darray)(float,float,float,float) {g}.xy, {q}.xyzx, t114.xyzw, s0, {f}.xyxx, {f}.zwzz
mad {g}.xy, {g}.xyxx, l(2.007874, 2.007874, 0.000000, 0.000000), l(-1.007874, -1.007874, 0.000000, 0.000000)
mul {h}.x, {s}.x, {g}.x
mad {h}.x, {t}.x, {g}.y, {h}.x
mul {h}.y, {s}.x, {g}.y
mad {h}.y, -{t}.x, {g}.x, {h}.y
mad {j}.xy, {h}.xyxx, {u}.xxxx, {j}.xyxx
add {h}.x, {a}.z, -{n}.y
add {h}.y, {a}.w, -{o}.y
mul {h}.z, {s}.y, {h}.x
mad {h}.z, -{t}.y, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.y
mul {h}.w, {t}.y, {h}.x
mad {h}.w, {s}.y, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.y
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.y, {d}.x
mad {f}.x, -{t}.y, {d}.y, {f}.x
mul {f}.y, {t}.y, {d}.x
mad {f}.y, {s}.y, {d}.y, {f}.y
mul {f}.z, {s}.y, {d}.z
mad {f}.z, -{t}.y, {d}.w, {f}.z
mul {f}.w, {t}.y, {d}.z
mad {f}.w, {s}.y, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.z, {q}.xyzx, t113.xyxy, s0, {f}.xyxx, {f}.zwzz
mad {e}.z, {g}.z, {v}.y, {e}.z
sample_d_indexable(texture2darray)(float,float,float,float) {g}.xy, {q}.xyzx, t114.xyzw, s0, {f}.xyxx, {f}.zwzz
mad {g}.xy, {g}.xyxx, l(2.007874, 2.007874, 0.000000, 0.000000), l(-1.007874, -1.007874, 0.000000, 0.000000)
mul {h}.x, {s}.y, {g}.x
mad {h}.x, {t}.y, {g}.y, {h}.x
mul {h}.y, {s}.y, {g}.y
mad {h}.y, -{t}.y, {g}.x, {h}.y
mad {j}.xy, {h}.xyxx, {u}.yyyy, {j}.xyxx
add {h}.x, {a}.z, -{n}.z
add {h}.y, {a}.w, -{o}.z
mul {h}.z, {s}.z, {h}.x
mad {h}.z, -{t}.z, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.z
mul {h}.w, {t}.z, {h}.x
mad {h}.w, {s}.z, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.z
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.z, {d}.x
mad {f}.x, -{t}.z, {d}.y, {f}.x
mul {f}.y, {t}.z, {d}.x
mad {f}.y, {s}.z, {d}.y, {f}.y
mul {f}.z, {s}.z, {d}.z
mad {f}.z, -{t}.z, {d}.w, {f}.z
mul {f}.w, {t}.z, {d}.z
mad {f}.w, {s}.z, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.z, {q}.xyzx, t113.xyxy, s0, {f}.xyxx, {f}.zwzz
mad {e}.z, {g}.z, {v}.z, {e}.z
sample_d_indexable(texture2darray)(float,float,float,float) {g}.xy, {q}.xyzx, t114.xyzw, s0, {f}.xyxx, {f}.zwzz
mad {g}.xy, {g}.xyxx, l(2.007874, 2.007874, 0.000000, 0.000000), l(-1.007874, -1.007874, 0.000000, 0.000000)
mul {h}.x, {s}.z, {g}.x
mad {h}.x, {t}.z, {g}.y, {h}.x
mul {h}.y, {s}.z, {g}.y
mad {h}.y, -{t}.z, {g}.x, {h}.y
mad {j}.xy, {h}.xyxx, {u}.zzzz, {j}.xyxx
div {j}.z, cb2[0].x, {b}.y
mul {j}.z, {j}.z, {i}.w
mul {j}.xy, {j}.xyxx, {j}.zzzz
mad {i}.x, {j}.x, {d}.x, {i}.x
mad {i}.x, {j}.y, {d}.y, {i}.x
mad {i}.y, {j}.x, {d}.z, {i}.y
mad {i}.y, {j}.y, {d}.w, {i}.y
mul {a}.zw, v1.xxxy, {b}.zzzz
deriv_rtx_fine {d}.xy, {a}.zwzz
deriv_rty_fine {d}.zw, {a}.zzzw
mad {a}.zw, {z}.xxxy, {b}.zzzz, {a}.xyzw
div {w}.x, {b}.z, {a}.x
mul {w}.y, {w}.x, l(0.57735027)
round_ne {w}.y, {w}.y
max {w}.y, {w}.y, l(1.000000)
add {w}.y, {w}.y, {w}.y
mul {w}.z, {w}.y, {a}.x
div {w}.w, {w}.z, {b}.z
mul {g}.y, {a}.w, {w}.w
mad {g}.x, {g}.y, l(-0.500000), {a}.z
round_ni {h}.xy, {g}.xyxx
add {g}.xy, {g}.xyxx, -{h}.xyxx
add {g}.z, {g}.x, {g}.y
lt {f}.w, l(1.000000), {g}.z
and {f}.w, {f}.w, l(1065353216)
mad {w}.w, {f}.w, l(2.000000), l(-1.000000)
add {g}.z, {g}.z, l(-1.000000)
mul {u}.x, {w}.w, {g}.z
mad {u}.y, -{w}.w, {g}.x, {f}.w
mad {u}.z, -{w}.w, {g}.y, {f}.w
add {w}.w, -{f}.w, l(1.000000)
add {n}.x, {h}.x, {f}.w
add {n}.y, {h}.x, {w}.w
mov {n}.z, {n}.x
add {o}.x, {h}.y, {f}.w
mov {o}.y, {o}.x
add {o}.z, {h}.y, {w}.w
div {f}.xyz, {o}.xyzx, {w}.yyyy
round_ni {f}.xyz, {f}.xyzx
mad {g}.xyz, -{f}.xyzx, {w}.yyyy, {o}.xyzx
mul {h}.xyz, {f}.xyzx, {w}.yyyy
mad {h}.xyz, {h}.xyzx, l(0.500000), {n}.xyzx
div {f}.xyz, {h}.xyzx, {w}.xxxx
round_ni {f}.xyz, {f}.xyzx
mad {h}.xyz, -{f}.xyzx, {w}.xxxx, {h}.xyzx
mad {p}.xyz, {h}.xyzx, {w}.yyyy, {g}.xyzx
ftoi {p}.xyz, {p}.xyzx
iadd {f}.xyz, {p}.xyzx, l(0x00000000, 0x00000000, 0x00000000, 0)
ushr {h}.xyz, {f}.xyzx, l(16, 16, 16, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
imul null, {f}.xyz, {f}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {f}.xyzx, l(15, 15, 15, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
imul null, {f}.xyz, {f}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {f}.xyzx, l(16, 16, 16, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
utof {f}.xyz, {f}.xyzx
mul {f}.xyz, {f}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
iadd {g}.xyz, {p}.xyzx, l(0x68e31da4, 0x68e31da4, 0x68e31da4, 0)
ushr {h}.xyz, {g}.xyzx, l(16, 16, 16, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
imul null, {g}.xyz, {g}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {g}.xyzx, l(15, 15, 15, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
imul null, {g}.xyz, {g}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {g}.xyzx, l(16, 16, 16, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
utof {g}.xyz, {g}.xyzx
mul {g}.xyz, {g}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
iadd {t}.xyz, {p}.xyzx, l(0xb5297a4d, 0xb5297a4d, 0xb5297a4d, 0)
ushr {h}.xyz, {t}.xyzx, l(16, 16, 16, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
imul null, {t}.xyz, {t}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {t}.xyzx, l(15, 15, 15, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
imul null, {t}.xyz, {t}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {t}.xyzx, l(16, 16, 16, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
utof {t}.xyz, {t}.xyzx
mul {t}.xyz, {t}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
div {w}.w, {b}.z, {w}.z
mad {n}.xyz, {o}.xyzx, l(0.500000), {n}.xyzx
mul {o}.xyz, {o}.xyzx, {w}.wwww
add {p}.xyz, {n}.xyzx, {f}.xyzx
add {r}.xyz, {o}.xyzx, {g}.xyzx
mul {h}.xyz, {t}.xyzx, l(6.28318531, 6.28318531, 6.28318531, 0.000000)
mul {g}.xyz, {t}.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000)
round_ni {g}.xyz, {g}.xyzx
mul {g}.xyz, {g}.xyzx, l(3.14159265, 3.14159265, 3.14159265, 0.000000)
eq {w}.x, {q}.z, l(0.000000)
movc {h}.xyz, {w}.xxxx, {g}.xyzx, {h}.xyzx
sincos {t}.x, {s}.x, {h}.x
sincos {t}.y, {s}.y, {h}.y
sincos {t}.z, {s}.z, {h}.z
mul {v}.xyz, {u}.xyzx, {u}.xyzx
mul {v}.xyz, {v}.xyzx, {v}.xyzx
dp3 {w}.x, {v}.xyzx, l(1.000000, 1.000000, 1.000000, 0.000000)
div {v}.xyz, {v}.xyzx, {w}.xxxx
dp3 {m}.z, {f}.xyzx, {v}.xyzx
lt {w}.x, {m}.w, l(0.000000)
movc {s}.xyz, {w}.xxxx, l(1.000000, 1.000000, 1.000000, 0.000000), {s}.xyzx
movc {t}.xyz, {w}.xxxx, l(0.000000, 0.000000, 0.000000, 0.000000), {t}.xyzx
movc {p}.xyz, {w}.xxxx, {n}.xyzx, {p}.xyzx
movc {r}.xyz, {w}.xxxx, {o}.xyzx, {r}.xyzx
movc {u}.xyz, {w}.xxxx, l(1.000000, 0.000000, 0.000000, 0.000000), {u}.xyzx
movc {v}.xyz, {w}.xxxx, l(1.000000, 0.000000, 0.000000, 0.000000), {v}.xyzx
mov {e}.y, l(0.000000)
mov {j}.xy, l(0.000000, 0.000000, 0.000000, 0.000000)
add {h}.x, {a}.z, -{n}.x
add {h}.y, {a}.w, -{o}.x
mul {h}.z, {s}.x, {h}.x
mad {h}.z, -{t}.x, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.x
mul {h}.w, {t}.x, {h}.x
mad {h}.w, {s}.x, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.x
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.x, {d}.x
mad {f}.x, -{t}.x, {d}.y, {f}.x
mul {f}.y, {t}.x, {d}.x
mad {f}.y, {s}.x, {d}.y, {f}.y
mul {f}.z, {s}.x, {d}.z
mad {f}.z, -{t}.x, {d}.w, {f}.z
mul {f}.w, {t}.x, {d}.z
mad {f}.w, {s}.x, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.y, {q}.xyzx, t110.xyzw, s0, {f}.xyxx, {f}.zwzz
mad {e}.y, {g}.y, {v}.x, {e}.y
sample_d_indexable(texture2darray)(float,float,float,float) {g}.xy, {q}.xyzx, t111.xyzw, s0, {f}.xyxx, {f}.zwzz
mad {g}.xy, {g}.xyxx, l(2.007874, 2.007874, 0.000000, 0.000000), l(-1.007874, -1.007874, 0.000000, 0.000000)
mul {h}.x, {s}.x, {g}.x
mad {h}.x, {t}.x, {g}.y, {h}.x
mul {h}.y, {s}.x, {g}.y
mad {h}.y, -{t}.x, {g}.x, {h}.y
mad {j}.xy, {h}.xyxx, {u}.xxxx, {j}.xyxx
add {h}.x, {a}.z, -{n}.y
add {h}.y, {a}.w, -{o}.y
mul {h}.z, {s}.y, {h}.x
mad {h}.z, -{t}.y, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.y
mul {h}.w, {t}.y, {h}.x
mad {h}.w, {s}.y, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.y
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.y, {d}.x
mad {f}.x, -{t}.y, {d}.y, {f}.x
mul {f}.y, {t}.y, {d}.x
mad {f}.y, {s}.y, {d}.y, {f}.y
mul {f}.z, {s}.y, {d}.z
mad {f}.z, -{t}.y, {d}.w, {f}.z
mul {f}.w, {t}.y, {d}.z
mad {f}.w, {s}.y, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.y, {q}.xyzx, t110.xyzw, s0, {f}.xyxx, {f}.zwzz
mad {e}.y, {g}.y, {v}.y, {e}.y
sample_d_indexable(texture2darray)(float,float,float,float) {g}.xy, {q}.xyzx, t111.xyzw, s0, {f}.xyxx, {f}.zwzz
mad {g}.xy, {g}.xyxx, l(2.007874, 2.007874, 0.000000, 0.000000), l(-1.007874, -1.007874, 0.000000, 0.000000)
mul {h}.x, {s}.y, {g}.x
mad {h}.x, {t}.y, {g}.y, {h}.x
mul {h}.y, {s}.y, {g}.y
mad {h}.y, -{t}.y, {g}.x, {h}.y
mad {j}.xy, {h}.xyxx, {u}.yyyy, {j}.xyxx
add {h}.x, {a}.z, -{n}.z
add {h}.y, {a}.w, -{o}.z
mul {h}.z, {s}.z, {h}.x
mad {h}.z, -{t}.z, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.z
mul {h}.w, {t}.z, {h}.x
mad {h}.w, {s}.z, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.z
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.z, {d}.x
mad {f}.x, -{t}.z, {d}.y, {f}.x
mul {f}.y, {t}.z, {d}.x
mad {f}.y, {s}.z, {d}.y, {f}.y
mul {f}.z, {s}.z, {d}.z
mad {f}.z, -{t}.z, {d}.w, {f}.z
mul {f}.w, {t}.z, {d}.z
mad {f}.w, {s}.z, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.y, {q}.xyzx, t110.xyzw, s0, {f}.xyxx, {f}.zwzz
mad {e}.y, {g}.y, {v}.z, {e}.y
sample_d_indexable(texture2darray)(float,float,float,float) {g}.xy, {q}.xyzx, t111.xyzw, s0, {f}.xyxx, {f}.zwzz
mad {g}.xy, {g}.xyxx, l(2.007874, 2.007874, 0.000000, 0.000000), l(-1.007874, -1.007874, 0.000000, 0.000000)
mul {h}.x, {s}.z, {g}.x
mad {h}.x, {t}.z, {g}.y, {h}.x
mul {h}.y, {s}.z, {g}.y
mad {h}.y, -{t}.z, {g}.x, {h}.y
mad {j}.xy, {h}.xyxx, {u}.zzzz, {j}.xyxx
div {j}.z, cb2[0].x, {b}.z
mul {j}.z, {j}.z, {i}.z
mul {j}.xy, {j}.xyxx, {j}.zzzz
mad {i}.x, {j}.x, {d}.x, {i}.x
mad {i}.x, {j}.y, {d}.y, {i}.x
mad {i}.y, {j}.x, {d}.z, {i}.y
mad {i}.y, {j}.y, {d}.w, {i}.y
mul {a}.zw, v1.xxxy, {b}.wwww
deriv_rtx_fine {d}.xy, {a}.zwzz
deriv_rty_fine {d}.zw, {a}.zzzw
mad {a}.zw, {z}.xxxy, {b}.wwww, {a}.xyzw
div {w}.x, {b}.w, {a}.x
mul {w}.y, {w}.x, l(0.57735027)
round_ne {w}.y, {w}.y
max {w}.y, {w}.y, l(1.000000)
add {w}.y, {w}.y, {w}.y
mul {w}.z, {w}.y, {a}.x
div {w}.w, {w}.z, {b}.w
mul {g}.y, {a}.w, {w}.w
mad {g}.x, {g}.y, l(-0.500000), {a}.z
round_ni {h}.xy, {g}.xyxx
add {g}.xy, {g}.xyxx, -{h}.xyxx
add {g}.z, {g}.x, {g}.y
lt {f}.w, l(1.000000), {g}.z
and {f}.w, {f}.w, l(1065353216)
mad {w}.w, {f}.w, l(2.000000), l(-1.000000)
add {g}.z, {g}.z, l(-1.000000)
mul {u}.x, {w}.w, {g}.z
mad {u}.y, -{w}.w, {g}.x, {f}.w
mad {u}.z, -{w}.w, {g}.y, {f}.w
add {w}.w, -{f}.w, l(1.000000)
add {n}.x, {h}.x, {f}.w
add {n}.y, {h}.x, {w}.w
mov {n}.z, {n}.x
add {o}.x, {h}.y, {f}.w
mov {o}.y, {o}.x
add {o}.z, {h}.y, {w}.w
div {f}.xyz, {o}.xyzx, {w}.yyyy
round_ni {f}.xyz, {f}.xyzx
mad {g}.xyz, -{f}.xyzx, {w}.yyyy, {o}.xyzx
mul {h}.xyz, {f}.xyzx, {w}.yyyy
mad {h}.xyz, {h}.xyzx, l(0.500000), {n}.xyzx
div {f}.xyz, {h}.xyzx, {w}.xxxx
round_ni {f}.xyz, {f}.xyzx
mad {h}.xyz, -{f}.xyzx, {w}.xxxx, {h}.xyzx
mad {p}.xyz, {h}.xyzx, {w}.yyyy, {g}.xyzx
ftoi {p}.xyz, {p}.xyzx
iadd {f}.xyz, {p}.xyzx, l(0x00000000, 0x00000000, 0x00000000, 0)
ushr {h}.xyz, {f}.xyzx, l(16, 16, 16, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
imul null, {f}.xyz, {f}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {f}.xyzx, l(15, 15, 15, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
imul null, {f}.xyz, {f}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {f}.xyzx, l(16, 16, 16, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
utof {f}.xyz, {f}.xyzx
mul {f}.xyz, {f}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
iadd {g}.xyz, {p}.xyzx, l(0x68e31da4, 0x68e31da4, 0x68e31da4, 0)
ushr {h}.xyz, {g}.xyzx, l(16, 16, 16, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
imul null, {g}.xyz, {g}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {g}.xyzx, l(15, 15, 15, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
imul null, {g}.xyz, {g}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {g}.xyzx, l(16, 16, 16, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
utof {g}.xyz, {g}.xyzx
mul {g}.xyz, {g}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
iadd {t}.xyz, {p}.xyzx, l(0xb5297a4d, 0xb5297a4d, 0xb5297a4d, 0)
ushr {h}.xyz, {t}.xyzx, l(16, 16, 16, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
imul null, {t}.xyz, {t}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {t}.xyzx, l(15, 15, 15, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
imul null, {t}.xyz, {t}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {t}.xyzx, l(16, 16, 16, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
utof {t}.xyz, {t}.xyzx
mul {t}.xyz, {t}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
div {w}.w, {b}.w, {w}.z
mad {n}.xyz, {o}.xyzx, l(0.500000), {n}.xyzx
mul {o}.xyz, {o}.xyzx, {w}.wwww
add {p}.xyz, {n}.xyzx, {f}.xyzx
add {r}.xyz, {o}.xyzx, {g}.xyzx
mul {h}.xyz, {t}.xyzx, l(6.28318531, 6.28318531, 6.28318531, 0.000000)
mul {g}.xyz, {t}.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000)
round_ni {g}.xyz, {g}.xyzx
mul {g}.xyz, {g}.xyzx, l(3.14159265, 3.14159265, 3.14159265, 0.000000)
eq {w}.x, {q}.z, l(0.000000)
movc {h}.xyz, {w}.xxxx, {g}.xyzx, {h}.xyzx
sincos {t}.x, {s}.x, {h}.x
sincos {t}.y, {s}.y, {h}.y
sincos {t}.z, {s}.z, {h}.z
mul {v}.xyz, {u}.xyzx, {u}.xyzx
mul {v}.xyz, {v}.xyzx, {v}.xyzx
dp3 {w}.x, {v}.xyzx, l(1.000000, 1.000000, 1.000000, 0.000000)
div {v}.xyz, {v}.xyzx, {w}.xxxx
lt {w}.x, {m}.w, l(0.000000)
movc {s}.xyz, {w}.xxxx, l(1.000000, 1.000000, 1.000000, 0.000000), {s}.xyzx
movc {t}.xyz, {w}.xxxx, l(0.000000, 0.000000, 0.000000, 0.000000), {t}.xyzx
movc {p}.xyz, {w}.xxxx, {n}.xyzx, {p}.xyzx
movc {r}.xyz, {w}.xxxx, {o}.xyzx, {r}.xyzx
movc {u}.xyz, {w}.xxxx, l(1.000000, 0.000000, 0.000000, 0.000000), {u}.xyzx
movc {v}.xyz, {w}.xxxx, l(1.000000, 0.000000, 0.000000, 0.000000), {v}.xyzx
mov {e}.w, l(0.000000)
add {h}.x, {a}.z, -{n}.x
add {h}.y, {a}.w, -{o}.x
mul {h}.z, {s}.x, {h}.x
mad {h}.z, -{t}.x, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.x
mul {h}.w, {t}.x, {h}.x
mad {h}.w, {s}.x, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.x
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.x, {d}.x
mad {f}.x, -{t}.x, {d}.y, {f}.x
mul {f}.y, {t}.x, {d}.x
mad {f}.y, {s}.x, {d}.y, {f}.y
mul {f}.z, {s}.x, {d}.z
mad {f}.z, -{t}.x, {d}.w, {f}.z
mul {f}.w, {t}.x, {d}.z
mad {f}.w, {s}.x, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.w, {q}.xyzx, t113.xyxy, s0, {f}.xyxx, {f}.zwzz
mad {e}.w, {g}.w, {v}.x, {e}.w
add {h}.x, {a}.z, -{n}.y
add {h}.y, {a}.w, -{o}.y
mul {h}.z, {s}.y, {h}.x
mad {h}.z, -{t}.y, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.y
mul {h}.w, {t}.y, {h}.x
mad {h}.w, {s}.y, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.y
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.y, {d}.x
mad {f}.x, -{t}.y, {d}.y, {f}.x
mul {f}.y, {t}.y, {d}.x
mad {f}.y, {s}.y, {d}.y, {f}.y
mul {f}.z, {s}.y, {d}.z
mad {f}.z, -{t}.y, {d}.w, {f}.z
mul {f}.w, {t}.y, {d}.z
mad {f}.w, {s}.y, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.w, {q}.xyzx, t113.xyxy, s0, {f}.xyxx, {f}.zwzz
mad {e}.w, {g}.w, {v}.y, {e}.w
add {h}.x, {a}.z, -{n}.z
add {h}.y, {a}.w, -{o}.z
mul {h}.z, {s}.z, {h}.x
mad {h}.z, -{t}.z, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.z
mul {h}.w, {t}.z, {h}.x
mad {h}.w, {s}.z, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.z
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.z, {d}.x
mad {f}.x, -{t}.z, {d}.y, {f}.x
mul {f}.y, {t}.z, {d}.x
mad {f}.y, {s}.z, {d}.y, {f}.y
mul {f}.z, {s}.z, {d}.z
mad {f}.z, -{t}.z, {d}.w, {f}.z
mul {f}.w, {t}.z, {d}.z
mad {f}.w, {s}.z, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.w, {q}.xyzx, t113.xyxy, s0, {f}.xyxx, {f}.zwzz
mad {e}.w, {g}.w, {v}.z, {e}.w
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(514, 0, 0, 0), t120.xyzw
ge {f}.x, {j}.x, l(1.500000)
lt {f}.y, {j}.x, l(3.500000)
and {f}.z, {f}.x, {f}.y
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(517, 0, 0, 0), t120.xyzw
eq {f}.x, {j}.x, l(0.000000)
movc {j}.x, {f}.x, l(0.500000), {j}.x
lt {f}.x, {j}.x, l(0.000000)
movc {j}.x, {f}.x, l(-10.000000), {j}.x
max {f}.x, {j}.x, l(0.020000)
min {f}.x, {f}.x, l(0.980000)
add {f}.y, -{f}.x, l(1.000000)
log {f}.x, {f}.x
log {f}.y, {f}.y
add {f}.x, {f}.x, -{f}.y
mad {f}.y, {f}.x, l(-0.002437424), l(-0.018739033)
mad {f}.y, {f}.y, {f}.x, l(-0.487268350)
mad {f}.y, {f}.y, {f}.x, l(0.264974522)
add {f}.x, {m}.x, -{f}.y
mad_sat {j}.w, {f}.x, l(0.400000), l(0.500000)
mad_sat {j}.w, {j}.w, l(4.000000), l(-0.200000)
mul {j}.w, {j}.w, l(1.000000)
and {j}.w, {j}.w, {f}.z
mad {k}.z, -{k}.z, {j}.w, {k}.z
mul {k}.w, {a}.y, l(0.125000)
round_ne {k}.w, {k}.w
max {k}.w, {k}.w, l(1.000000)
mul {k}.w, {k}.w, {a}.x
mul {a}.zw, v1.xxxy, {k}.wwww
deriv_rtx_fine {d}.xy, {a}.zwzz
deriv_rty_fine {d}.zw, {a}.zzzw
mad {a}.zw, {z}.xxxy, {k}.wwww, {a}.xyzw
div {w}.x, {k}.w, {a}.x
mul {w}.y, {w}.x, l(0.57735027)
round_ne {w}.y, {w}.y
max {w}.y, {w}.y, l(1.000000)
add {w}.y, {w}.y, {w}.y
mul {w}.z, {w}.y, {a}.x
div {w}.w, {w}.z, {k}.w
mul {g}.y, {a}.w, {w}.w
mad {g}.x, {g}.y, l(-0.500000), {a}.z
round_ni {h}.xy, {g}.xyxx
add {g}.xy, {g}.xyxx, -{h}.xyxx
add {g}.z, {g}.x, {g}.y
lt {f}.w, l(1.000000), {g}.z
and {f}.w, {f}.w, l(1065353216)
mad {w}.w, {f}.w, l(2.000000), l(-1.000000)
add {g}.z, {g}.z, l(-1.000000)
mul {u}.x, {w}.w, {g}.z
mad {u}.y, -{w}.w, {g}.x, {f}.w
mad {u}.z, -{w}.w, {g}.y, {f}.w
add {w}.w, -{f}.w, l(1.000000)
add {n}.x, {h}.x, {f}.w
add {n}.y, {h}.x, {w}.w
mov {n}.z, {n}.x
add {o}.x, {h}.y, {f}.w
mov {o}.y, {o}.x
add {o}.z, {h}.y, {w}.w
div {f}.xyz, {o}.xyzx, {w}.yyyy
round_ni {f}.xyz, {f}.xyzx
mad {g}.xyz, -{f}.xyzx, {w}.yyyy, {o}.xyzx
mul {h}.xyz, {f}.xyzx, {w}.yyyy
mad {h}.xyz, {h}.xyzx, l(0.500000), {n}.xyzx
div {f}.xyz, {h}.xyzx, {w}.xxxx
round_ni {f}.xyz, {f}.xyzx
mad {h}.xyz, -{f}.xyzx, {w}.xxxx, {h}.xyzx
mad {p}.xyz, {h}.xyzx, {w}.yyyy, {g}.xyzx
ftoi {p}.xyz, {p}.xyzx
iadd {f}.xyz, {p}.xyzx, l(0x00000000, 0x00000000, 0x00000000, 0)
ushr {h}.xyz, {f}.xyzx, l(16, 16, 16, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
imul null, {f}.xyz, {f}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {f}.xyzx, l(15, 15, 15, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
imul null, {f}.xyz, {f}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {f}.xyzx, l(16, 16, 16, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
utof {f}.xyz, {f}.xyzx
mul {f}.xyz, {f}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
iadd {g}.xyz, {p}.xyzx, l(0x68e31da4, 0x68e31da4, 0x68e31da4, 0)
ushr {h}.xyz, {g}.xyzx, l(16, 16, 16, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
imul null, {g}.xyz, {g}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {g}.xyzx, l(15, 15, 15, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
imul null, {g}.xyz, {g}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {g}.xyzx, l(16, 16, 16, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
utof {g}.xyz, {g}.xyzx
mul {g}.xyz, {g}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
iadd {t}.xyz, {p}.xyzx, l(0xb5297a4d, 0xb5297a4d, 0xb5297a4d, 0)
ushr {h}.xyz, {t}.xyzx, l(16, 16, 16, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
imul null, {t}.xyz, {t}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {t}.xyzx, l(15, 15, 15, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
imul null, {t}.xyz, {t}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {t}.xyzx, l(16, 16, 16, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
utof {t}.xyz, {t}.xyzx
mul {t}.xyz, {t}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
div {w}.w, {k}.w, {w}.z
mad {n}.xyz, {o}.xyzx, l(0.500000), {n}.xyzx
mul {o}.xyz, {o}.xyzx, {w}.wwww
add {p}.xyz, {n}.xyzx, {f}.xyzx
add {r}.xyz, {o}.xyzx, {g}.xyzx
mul {h}.xyz, {t}.xyzx, l(6.28318531, 6.28318531, 6.28318531, 0.000000)
mul {g}.xyz, {t}.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000)
round_ni {g}.xyz, {g}.xyzx
mul {g}.xyz, {g}.xyzx, l(3.14159265, 3.14159265, 3.14159265, 0.000000)
eq {w}.x, {q}.z, l(0.000000)
movc {h}.xyz, {w}.xxxx, {g}.xyzx, {h}.xyzx
sincos {t}.x, {s}.x, {h}.x
sincos {t}.y, {s}.y, {h}.y
sincos {t}.z, {s}.z, {h}.z
mul {v}.xyz, {u}.xyzx, {u}.xyzx
mul {v}.xyz, {v}.xyzx, {v}.xyzx
dp3 {w}.x, {v}.xyzx, l(1.000000, 1.000000, 1.000000, 0.000000)
div {v}.xyz, {v}.xyzx, {w}.xxxx
lt {w}.x, {m}.w, l(0.000000)
movc {s}.xyz, {w}.xxxx, l(1.000000, 1.000000, 1.000000, 0.000000), {s}.xyzx
movc {t}.xyz, {w}.xxxx, l(0.000000, 0.000000, 0.000000, 0.000000), {t}.xyzx
movc {p}.xyz, {w}.xxxx, {n}.xyzx, {p}.xyzx
movc {r}.xyz, {w}.xxxx, {o}.xyzx, {r}.xyzx
movc {u}.xyz, {w}.xxxx, l(1.000000, 0.000000, 0.000000, 0.000000), {u}.xyzx
movc {v}.xyz, {w}.xxxx, l(1.000000, 0.000000, 0.000000, 0.000000), {v}.xyzx
mov {j}.xyz, l(0.000000, 0.000000, 0.000000, 0.000000)
add {h}.x, {a}.z, -{n}.x
add {h}.y, {a}.w, -{o}.x
mul {h}.z, {s}.x, {h}.x
mad {h}.z, -{t}.x, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.x
mul {h}.w, {t}.x, {h}.x
mad {h}.w, {s}.x, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.x
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.x, {d}.x
mad {f}.x, -{t}.x, {d}.y, {f}.x
mul {f}.y, {t}.x, {d}.x
mad {f}.y, {s}.x, {d}.y, {f}.y
mul {f}.z, {s}.x, {d}.z
mad {f}.z, -{t}.x, {d}.w, {f}.z
mul {f}.w, {t}.x, {d}.z
mad {f}.w, {s}.x, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.x, {q}.xyzx, t115.xyzw, s0, {f}.xyxx, {f}.zwzz
mad {j}.x, {g}.x, {v}.x, {j}.x
sample_d_indexable(texture2darray)(float,float,float,float) {g}.xy, {q}.xyzx, t112.xyxx, s0, {f}.xyxx, {f}.zwzz
mad {g}.xy, {g}.xyxx, l(2.007874, 2.007874, 0.000000, 0.000000), l(-1.007874, -1.007874, 0.000000, 0.000000)
mul {h}.x, {s}.x, {g}.x
mad {h}.x, {t}.x, {g}.y, {h}.x
mul {h}.y, {s}.x, {g}.y
mad {h}.y, -{t}.x, {g}.x, {h}.y
mad {j}.yz, {h}.xxyx, {u}.xxxx, {j}.yyzy
add {h}.x, {a}.z, -{n}.y
add {h}.y, {a}.w, -{o}.y
mul {h}.z, {s}.y, {h}.x
mad {h}.z, -{t}.y, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.y
mul {h}.w, {t}.y, {h}.x
mad {h}.w, {s}.y, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.y
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.y, {d}.x
mad {f}.x, -{t}.y, {d}.y, {f}.x
mul {f}.y, {t}.y, {d}.x
mad {f}.y, {s}.y, {d}.y, {f}.y
mul {f}.z, {s}.y, {d}.z
mad {f}.z, -{t}.y, {d}.w, {f}.z
mul {f}.w, {t}.y, {d}.z
mad {f}.w, {s}.y, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.x, {q}.xyzx, t115.xyzw, s0, {f}.xyxx, {f}.zwzz
mad {j}.x, {g}.x, {v}.y, {j}.x
sample_d_indexable(texture2darray)(float,float,float,float) {g}.xy, {q}.xyzx, t112.xyxx, s0, {f}.xyxx, {f}.zwzz
mad {g}.xy, {g}.xyxx, l(2.007874, 2.007874, 0.000000, 0.000000), l(-1.007874, -1.007874, 0.000000, 0.000000)
mul {h}.x, {s}.y, {g}.x
mad {h}.x, {t}.y, {g}.y, {h}.x
mul {h}.y, {s}.y, {g}.y
mad {h}.y, -{t}.y, {g}.x, {h}.y
mad {j}.yz, {h}.xxyx, {u}.yyyy, {j}.yyzy
add {h}.x, {a}.z, -{n}.z
add {h}.y, {a}.w, -{o}.z
mul {h}.z, {s}.z, {h}.x
mad {h}.z, -{t}.z, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.z
mul {h}.w, {t}.z, {h}.x
mad {h}.w, {s}.z, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.z
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.z, {d}.x
mad {f}.x, -{t}.z, {d}.y, {f}.x
mul {f}.y, {t}.z, {d}.x
mad {f}.y, {s}.z, {d}.y, {f}.y
mul {f}.z, {s}.z, {d}.z
mad {f}.z, -{t}.z, {d}.w, {f}.z
mul {f}.w, {t}.z, {d}.z
mad {f}.w, {s}.z, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.x, {q}.xyzx, t115.xyzw, s0, {f}.xyxx, {f}.zwzz
mad {j}.x, {g}.x, {v}.z, {j}.x
sample_d_indexable(texture2darray)(float,float,float,float) {g}.xy, {q}.xyzx, t112.xyxx, s0, {f}.xyxx, {f}.zwzz
mad {g}.xy, {g}.xyxx, l(2.007874, 2.007874, 0.000000, 0.000000), l(-1.007874, -1.007874, 0.000000, 0.000000)
mul {h}.x, {s}.z, {g}.x
mad {h}.x, {t}.z, {g}.y, {h}.x
mul {h}.y, {s}.z, {g}.y
mad {h}.y, -{t}.z, {g}.x, {h}.y
mad {j}.yz, {h}.xxyx, {u}.zzzz, {j}.yyzy
add {k}.x, {k}.x, {j}.x
add {k}.x, {k}.x, l(-0.500000)
div {k}.y, cb2[0].x, {k}.w
mul {k}.y, {k}.y, {k}.z
mul {j}.yz, {j}.yyzy, {k}.yyyy
mad {i}.x, {j}.y, {d}.x, {i}.x
mad {i}.x, {j}.z, {d}.y, {i}.x
mad {i}.y, {j}.y, {d}.z, {i}.y
mad {i}.y, {j}.z, {d}.w, {i}.y
mul {k}.w, {a}.y, l(0.090909)
round_ne {k}.w, {k}.w
max {k}.w, {k}.w, l(1.000000)
mul {k}.w, {k}.w, {a}.x
mul {a}.zw, v1.xxyx, {k}.wwww
deriv_rtx_fine {d}.xy, {a}.zwzz
deriv_rty_fine {d}.zw, {a}.zzzw
mad {a}.zw, {z}.xxyx, {k}.wwww, {a}.xyzw
div {w}.x, {k}.w, {a}.x
mul {w}.y, {w}.x, l(0.57735027)
round_ne {w}.y, {w}.y
max {w}.y, {w}.y, l(1.000000)
add {w}.y, {w}.y, {w}.y
mul {w}.z, {w}.y, {a}.x
div {w}.w, {w}.z, {k}.w
mul {g}.y, {a}.w, {w}.w
mad {g}.x, {g}.y, l(-0.500000), {a}.z
round_ni {h}.xy, {g}.xyxx
add {g}.xy, {g}.xyxx, -{h}.xyxx
add {g}.z, {g}.x, {g}.y
lt {f}.w, l(1.000000), {g}.z
and {f}.w, {f}.w, l(1065353216)
mad {w}.w, {f}.w, l(2.000000), l(-1.000000)
add {g}.z, {g}.z, l(-1.000000)
mul {u}.x, {w}.w, {g}.z
mad {u}.y, -{w}.w, {g}.x, {f}.w
mad {u}.z, -{w}.w, {g}.y, {f}.w
add {w}.w, -{f}.w, l(1.000000)
add {n}.x, {h}.x, {f}.w
add {n}.y, {h}.x, {w}.w
mov {n}.z, {n}.x
add {o}.x, {h}.y, {f}.w
mov {o}.y, {o}.x
add {o}.z, {h}.y, {w}.w
div {f}.xyz, {o}.xyzx, {w}.yyyy
round_ni {f}.xyz, {f}.xyzx
mad {g}.xyz, -{f}.xyzx, {w}.yyyy, {o}.xyzx
mul {h}.xyz, {f}.xyzx, {w}.yyyy
mad {h}.xyz, {h}.xyzx, l(0.500000), {n}.xyzx
div {f}.xyz, {h}.xyzx, {w}.xxxx
round_ni {f}.xyz, {f}.xyzx
mad {h}.xyz, -{f}.xyzx, {w}.xxxx, {h}.xyzx
mad {p}.xyz, {h}.xyzx, {w}.yyyy, {g}.xyzx
ftoi {p}.xyz, {p}.xyzx
iadd {f}.xyz, {p}.xyzx, l(0x00000000, 0x00000000, 0x00000000, 0)
ushr {h}.xyz, {f}.xyzx, l(16, 16, 16, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
imul null, {f}.xyz, {f}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {f}.xyzx, l(15, 15, 15, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
imul null, {f}.xyz, {f}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {f}.xyzx, l(16, 16, 16, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
utof {f}.xyz, {f}.xyzx
mul {f}.xyz, {f}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
iadd {g}.xyz, {p}.xyzx, l(0x68e31da4, 0x68e31da4, 0x68e31da4, 0)
ushr {h}.xyz, {g}.xyzx, l(16, 16, 16, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
imul null, {g}.xyz, {g}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {g}.xyzx, l(15, 15, 15, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
imul null, {g}.xyz, {g}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {g}.xyzx, l(16, 16, 16, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
utof {g}.xyz, {g}.xyzx
mul {g}.xyz, {g}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
iadd {t}.xyz, {p}.xyzx, l(0xb5297a4d, 0xb5297a4d, 0xb5297a4d, 0)
ushr {h}.xyz, {t}.xyzx, l(16, 16, 16, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
imul null, {t}.xyz, {t}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {t}.xyzx, l(15, 15, 15, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
imul null, {t}.xyz, {t}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {t}.xyzx, l(16, 16, 16, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
utof {t}.xyz, {t}.xyzx
mul {t}.xyz, {t}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
div {w}.w, {k}.w, {w}.z
mad {n}.xyz, {o}.xyzx, l(0.500000), {n}.xyzx
mul {o}.xyz, {o}.xyzx, {w}.wwww
add {p}.xyz, {n}.xyzx, {f}.xyzx
add {r}.xyz, {o}.xyzx, {g}.xyzx
mul {h}.xyz, {t}.xyzx, l(6.28318531, 6.28318531, 6.28318531, 0.000000)
mul {g}.xyz, {t}.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000)
round_ni {g}.xyz, {g}.xyzx
mul {g}.xyz, {g}.xyzx, l(3.14159265, 3.14159265, 3.14159265, 0.000000)
eq {w}.x, {q}.z, l(0.000000)
movc {h}.xyz, {w}.xxxx, {g}.xyzx, {h}.xyzx
sincos {t}.x, {s}.x, {h}.x
sincos {t}.y, {s}.y, {h}.y
sincos {t}.z, {s}.z, {h}.z
mul {v}.xyz, {u}.xyzx, {u}.xyzx
mul {v}.xyz, {v}.xyzx, {v}.xyzx
dp3 {w}.x, {v}.xyzx, l(1.000000, 1.000000, 1.000000, 0.000000)
div {v}.xyz, {v}.xyzx, {w}.xxxx
lt {w}.x, {m}.w, l(0.000000)
movc {s}.xyz, {w}.xxxx, l(1.000000, 1.000000, 1.000000, 0.000000), {s}.xyzx
movc {t}.xyz, {w}.xxxx, l(0.000000, 0.000000, 0.000000, 0.000000), {t}.xyzx
movc {p}.xyz, {w}.xxxx, {n}.xyzx, {p}.xyzx
movc {r}.xyz, {w}.xxxx, {o}.xyzx, {r}.xyzx
movc {u}.xyz, {w}.xxxx, l(1.000000, 0.000000, 0.000000, 0.000000), {u}.xyzx
movc {v}.xyz, {w}.xxxx, l(1.000000, 0.000000, 0.000000, 0.000000), {v}.xyzx
mov {j}.xyz, l(0.000000, 0.000000, 0.000000, 0.000000)
add {h}.x, {a}.z, -{n}.x
add {h}.y, {a}.w, -{o}.x
mul {h}.z, {s}.x, {h}.x
mad {h}.z, -{t}.x, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.x
mul {h}.w, {t}.x, {h}.x
mad {h}.w, {s}.x, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.x
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.x, {d}.x
mad {f}.x, -{t}.x, {d}.y, {f}.x
mul {f}.y, {t}.x, {d}.x
mad {f}.y, {s}.x, {d}.y, {f}.y
mul {f}.z, {s}.x, {d}.z
mad {f}.z, -{t}.x, {d}.w, {f}.z
mul {f}.w, {t}.x, {d}.z
mad {f}.w, {s}.x, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.x, {q}.xyzx, t115.xyzw, s0, {f}.xyxx, {f}.zwzz
mad {j}.x, {g}.x, {v}.x, {j}.x
sample_d_indexable(texture2darray)(float,float,float,float) {g}.xy, {q}.xyzx, t112.xyxx, s0, {f}.xyxx, {f}.zwzz
mad {g}.xy, {g}.xyxx, l(2.007874, 2.007874, 0.000000, 0.000000), l(-1.007874, -1.007874, 0.000000, 0.000000)
mul {h}.x, {s}.x, {g}.x
mad {h}.x, {t}.x, {g}.y, {h}.x
mul {h}.y, {s}.x, {g}.y
mad {h}.y, -{t}.x, {g}.x, {h}.y
mad {j}.yz, {h}.xxyx, {u}.xxxx, {j}.yyzy
add {h}.x, {a}.z, -{n}.y
add {h}.y, {a}.w, -{o}.y
mul {h}.z, {s}.y, {h}.x
mad {h}.z, -{t}.y, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.y
mul {h}.w, {t}.y, {h}.x
mad {h}.w, {s}.y, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.y
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.y, {d}.x
mad {f}.x, -{t}.y, {d}.y, {f}.x
mul {f}.y, {t}.y, {d}.x
mad {f}.y, {s}.y, {d}.y, {f}.y
mul {f}.z, {s}.y, {d}.z
mad {f}.z, -{t}.y, {d}.w, {f}.z
mul {f}.w, {t}.y, {d}.z
mad {f}.w, {s}.y, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.x, {q}.xyzx, t115.xyzw, s0, {f}.xyxx, {f}.zwzz
mad {j}.x, {g}.x, {v}.y, {j}.x
sample_d_indexable(texture2darray)(float,float,float,float) {g}.xy, {q}.xyzx, t112.xyxx, s0, {f}.xyxx, {f}.zwzz
mad {g}.xy, {g}.xyxx, l(2.007874, 2.007874, 0.000000, 0.000000), l(-1.007874, -1.007874, 0.000000, 0.000000)
mul {h}.x, {s}.y, {g}.x
mad {h}.x, {t}.y, {g}.y, {h}.x
mul {h}.y, {s}.y, {g}.y
mad {h}.y, -{t}.y, {g}.x, {h}.y
mad {j}.yz, {h}.xxyx, {u}.yyyy, {j}.yyzy
add {h}.x, {a}.z, -{n}.z
add {h}.y, {a}.w, -{o}.z
mul {h}.z, {s}.z, {h}.x
mad {h}.z, -{t}.z, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.z
mul {h}.w, {t}.z, {h}.x
mad {h}.w, {s}.z, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.z
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.z, {d}.x
mad {f}.x, -{t}.z, {d}.y, {f}.x
mul {f}.y, {t}.z, {d}.x
mad {f}.y, {s}.z, {d}.y, {f}.y
mul {f}.z, {s}.z, {d}.z
mad {f}.z, -{t}.z, {d}.w, {f}.z
mul {f}.w, {t}.z, {d}.z
mad {f}.w, {s}.z, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.x, {q}.xyzx, t115.xyzw, s0, {f}.xyxx, {f}.zwzz
mad {j}.x, {g}.x, {v}.z, {j}.x
sample_d_indexable(texture2darray)(float,float,float,float) {g}.xy, {q}.xyzx, t112.xyxx, s0, {f}.xyxx, {f}.zwzz
mad {g}.xy, {g}.xyxx, l(2.007874, 2.007874, 0.000000, 0.000000), l(-1.007874, -1.007874, 0.000000, 0.000000)
mul {h}.x, {s}.z, {g}.x
mad {h}.x, {t}.z, {g}.y, {h}.x
mul {h}.y, {s}.z, {g}.y
mad {h}.y, -{t}.z, {g}.x, {h}.y
mad {j}.yz, {h}.xxyx, {u}.zzzz, {j}.yyzy
add {k}.x, {k}.x, {j}.x
add {k}.x, {k}.x, l(-0.500000)
div {k}.y, cb2[0].x, {k}.w
mul {k}.y, {k}.y, {k}.z
mul {j}.yz, {j}.yyzy, {k}.yyyy
mad {i}.x, {j}.y, {d}.x, {i}.x
mad {i}.x, {j}.z, {d}.y, {i}.x
mad {i}.y, {j}.y, {d}.z, {i}.y
mad {i}.y, {j}.z, {d}.w, {i}.y
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(514, 0, 0, 0), t120.xyzw
ge {f}.x, {j}.x, l(1.500000)
lt {f}.y, {j}.x, l(3.500000)
and {f}.x, {f}.x, {f}.y
if_nz {f}.x
mul {k}.w, {a}.y, l(0.250000)
round_ne {k}.w, {k}.w
max {k}.w, {k}.w, l(1.000000)
mul {k}.w, {k}.w, {a}.x
mul {a}.zw, v1.xxxy, {k}.wwww
deriv_rtx_fine {d}.xy, {a}.zwzz
deriv_rty_fine {d}.zw, {a}.zzzw
mad {a}.zw, {z}.xxxy, {k}.wwww, {a}.xyzw
div {w}.x, {k}.w, {a}.x
mul {w}.y, {w}.x, l(0.57735027)
round_ne {w}.y, {w}.y
max {w}.y, {w}.y, l(1.000000)
add {w}.y, {w}.y, {w}.y
mul {w}.z, {w}.y, {a}.x
div {w}.w, {w}.z, {k}.w
mul {g}.y, {a}.w, {w}.w
mad {g}.x, {g}.y, l(-0.500000), {a}.z
round_ni {h}.xy, {g}.xyxx
add {g}.xy, {g}.xyxx, -{h}.xyxx
add {g}.z, {g}.x, {g}.y
lt {f}.w, l(1.000000), {g}.z
and {f}.w, {f}.w, l(1065353216)
mad {w}.w, {f}.w, l(2.000000), l(-1.000000)
add {g}.z, {g}.z, l(-1.000000)
mul {u}.x, {w}.w, {g}.z
mad {u}.y, -{w}.w, {g}.x, {f}.w
mad {u}.z, -{w}.w, {g}.y, {f}.w
add {w}.w, -{f}.w, l(1.000000)
add {n}.x, {h}.x, {f}.w
add {n}.y, {h}.x, {w}.w
mov {n}.z, {n}.x
add {o}.x, {h}.y, {f}.w
mov {o}.y, {o}.x
add {o}.z, {h}.y, {w}.w
div {f}.xyz, {o}.xyzx, {w}.yyyy
round_ni {f}.xyz, {f}.xyzx
mad {g}.xyz, -{f}.xyzx, {w}.yyyy, {o}.xyzx
mul {h}.xyz, {f}.xyzx, {w}.yyyy
mad {h}.xyz, {h}.xyzx, l(0.500000), {n}.xyzx
div {f}.xyz, {h}.xyzx, {w}.xxxx
round_ni {f}.xyz, {f}.xyzx
mad {h}.xyz, -{f}.xyzx, {w}.xxxx, {h}.xyzx
mad {p}.xyz, {h}.xyzx, {w}.yyyy, {g}.xyzx
ftoi {p}.xyz, {p}.xyzx
iadd {f}.xyz, {p}.xyzx, l(0x00000000, 0x00000000, 0x00000000, 0)
ushr {h}.xyz, {f}.xyzx, l(16, 16, 16, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
imul null, {f}.xyz, {f}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {f}.xyzx, l(15, 15, 15, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
imul null, {f}.xyz, {f}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {f}.xyzx, l(16, 16, 16, 0)
xor {f}.xyz, {f}.xyzx, {h}.xyzx
utof {f}.xyz, {f}.xyzx
mul {f}.xyz, {f}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
iadd {g}.xyz, {p}.xyzx, l(0x68e31da4, 0x68e31da4, 0x68e31da4, 0)
ushr {h}.xyz, {g}.xyzx, l(16, 16, 16, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
imul null, {g}.xyz, {g}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {g}.xyzx, l(15, 15, 15, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
imul null, {g}.xyz, {g}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {g}.xyzx, l(16, 16, 16, 0)
xor {g}.xyz, {g}.xyzx, {h}.xyzx
utof {g}.xyz, {g}.xyzx
mul {g}.xyz, {g}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
iadd {t}.xyz, {p}.xyzx, l(0xb5297a4d, 0xb5297a4d, 0xb5297a4d, 0)
ushr {h}.xyz, {t}.xyzx, l(16, 16, 16, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
imul null, {t}.xyz, {t}.xyzx, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0)
ushr {h}.xyz, {t}.xyzx, l(15, 15, 15, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
imul null, {t}.xyz, {t}.xyzx, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0)
ushr {h}.xyz, {t}.xyzx, l(16, 16, 16, 0)
xor {t}.xyz, {t}.xyzx, {h}.xyzx
utof {t}.xyz, {t}.xyzx
mul {t}.xyz, {t}.xyzx, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 0.000000)
div {w}.w, {k}.w, {w}.z
mad {n}.xyz, {o}.xyzx, l(0.500000), {n}.xyzx
mul {o}.xyz, {o}.xyzx, {w}.wwww
add {p}.xyz, {n}.xyzx, {f}.xyzx
add {r}.xyz, {o}.xyzx, {g}.xyzx
mul {h}.xyz, {t}.xyzx, l(6.28318531, 6.28318531, 6.28318531, 0.000000)
mul {g}.xyz, {t}.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000)
round_ni {g}.xyz, {g}.xyzx
mul {g}.xyz, {g}.xyzx, l(3.14159265, 3.14159265, 3.14159265, 0.000000)
eq {w}.x, {q}.z, l(0.000000)
movc {h}.xyz, {w}.xxxx, {g}.xyzx, {h}.xyzx
sincos {t}.x, {s}.x, {h}.x
sincos {t}.y, {s}.y, {h}.y
sincos {t}.z, {s}.z, {h}.z
mul {v}.xyz, {u}.xyzx, {u}.xyzx
mul {v}.xyz, {v}.xyzx, {v}.xyzx
dp3 {w}.x, {v}.xyzx, l(1.000000, 1.000000, 1.000000, 0.000000)
div {v}.xyz, {v}.xyzx, {w}.xxxx
lt {w}.x, {m}.w, l(0.000000)
movc {s}.xyz, {w}.xxxx, l(1.000000, 1.000000, 1.000000, 0.000000), {s}.xyzx
movc {t}.xyz, {w}.xxxx, l(0.000000, 0.000000, 0.000000, 0.000000), {t}.xyzx
movc {p}.xyz, {w}.xxxx, {n}.xyzx, {p}.xyzx
movc {r}.xyz, {w}.xxxx, {o}.xyzx, {r}.xyzx
movc {u}.xyz, {w}.xxxx, l(1.000000, 0.000000, 0.000000, 0.000000), {u}.xyzx
movc {v}.xyz, {w}.xxxx, l(1.000000, 0.000000, 0.000000, 0.000000), {v}.xyzx
mov {q}.z, l(0.000000)
mov {j}.xyz, l(0.000000, 0.000000, 0.000000, 0.000000)
mov {b}.xy, l(0.000000, 0.000000, 0.000000, 0.000000)
add {h}.x, {a}.z, -{n}.x
add {h}.y, {a}.w, -{o}.x
mul {h}.z, {s}.x, {h}.x
mad {h}.z, -{t}.x, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.x
mul {h}.w, {t}.x, {h}.x
mad {h}.w, {s}.x, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.x
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.x, {d}.x
mad {f}.x, -{t}.x, {d}.y, {f}.x
mul {f}.y, {t}.x, {d}.x
mad {f}.y, {s}.x, {d}.y, {f}.y
mul {f}.z, {s}.x, {d}.z
mad {f}.z, -{t}.x, {d}.w, {f}.z
mul {f}.w, {t}.x, {d}.z
mad {f}.w, {s}.x, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.xy, {q}.xyzx, t116.xyxx, s0, {f}.xyxx, {f}.zwzz
mad {g}.xy, {g}.xyxx, l(3.011811, 3.011811, 0.000000, 0.000000), l(-1.511811, -1.511811, 0.000000, 0.000000)
mul {h}.x, {s}.x, {g}.x
mad {h}.x, {t}.x, {g}.y, {h}.x
mul {h}.y, {s}.x, {g}.y
mad {h}.y, -{t}.x, {g}.x, {h}.y
mad {j}.xy, {h}.xyxx, {u}.xxxx, {j}.xyxx
sample_d_indexable(texture2darray)(float,float,float,float) {g}.zw, {q}.xyzx, t117.xxxy, s0, {f}.xyxx, {f}.zwzz
mad {j}.z, {g}.z, {v}.x, {j}.z
max {h}.x, |{f}.x|, |{f}.y|
max {h}.y, |{f}.z|, |{f}.w|
max {h}.x, {h}.x, {h}.y
max {b}.z, {h}.x, l(0.000488281)
add {h}.x, {q}.x, {b}.z
mov {h}.y, {q}.y
frc {h}.xy, {h}.xyxx
mov {h}.z, {q}.z
sample_d_indexable(texture2darray)(float,float,float,float) {h}.w, {h}.xyzx, t117.xxxy, s0, {f}.xyxx, {f}.zwzz
add {h}.w, {h}.w, -{g}.w
mov {h}.x, {q}.x
add {h}.y, {q}.y, {b}.z
frc {h}.xy, {h}.xyxx
sample_d_indexable(texture2darray)(float,float,float,float) {g}.z, {h}.xyzx, t117.xxyx, s0, {f}.xyxx, {f}.zwzz
add {g}.z, {g}.z, -{g}.w
mul {h}.x, {h}.w, {f}.x
mad {h}.x, {g}.z, {f}.y, {h}.x
mul {h}.y, {h}.w, {f}.z
mad {h}.y, {g}.z, {f}.w, {h}.y
div {g}.x, {u}.x, {b}.z
mad {b}.xy, {h}.xyxx, {g}.xxxx, {b}.xyxx
add {h}.x, {a}.z, -{n}.y
add {h}.y, {a}.w, -{o}.y
mul {h}.z, {s}.y, {h}.x
mad {h}.z, -{t}.y, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.y
mul {h}.w, {t}.y, {h}.x
mad {h}.w, {s}.y, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.y
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.y, {d}.x
mad {f}.x, -{t}.y, {d}.y, {f}.x
mul {f}.y, {t}.y, {d}.x
mad {f}.y, {s}.y, {d}.y, {f}.y
mul {f}.z, {s}.y, {d}.z
mad {f}.z, -{t}.y, {d}.w, {f}.z
mul {f}.w, {t}.y, {d}.z
mad {f}.w, {s}.y, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.xy, {q}.xyzx, t116.xyxx, s0, {f}.xyxx, {f}.zwzz
mad {g}.xy, {g}.xyxx, l(3.011811, 3.011811, 0.000000, 0.000000), l(-1.511811, -1.511811, 0.000000, 0.000000)
mul {h}.x, {s}.y, {g}.x
mad {h}.x, {t}.y, {g}.y, {h}.x
mul {h}.y, {s}.y, {g}.y
mad {h}.y, -{t}.y, {g}.x, {h}.y
mad {j}.xy, {h}.xyxx, {u}.yyyy, {j}.xyxx
sample_d_indexable(texture2darray)(float,float,float,float) {g}.zw, {q}.xyzx, t117.xxxy, s0, {f}.xyxx, {f}.zwzz
mad {j}.z, {g}.z, {v}.y, {j}.z
max {h}.x, |{f}.x|, |{f}.y|
max {h}.y, |{f}.z|, |{f}.w|
max {h}.x, {h}.x, {h}.y
max {b}.z, {h}.x, l(0.000488281)
add {h}.x, {q}.x, {b}.z
mov {h}.y, {q}.y
frc {h}.xy, {h}.xyxx
mov {h}.z, {q}.z
sample_d_indexable(texture2darray)(float,float,float,float) {h}.w, {h}.xyzx, t117.xxxy, s0, {f}.xyxx, {f}.zwzz
add {h}.w, {h}.w, -{g}.w
mov {h}.x, {q}.x
add {h}.y, {q}.y, {b}.z
frc {h}.xy, {h}.xyxx
sample_d_indexable(texture2darray)(float,float,float,float) {g}.z, {h}.xyzx, t117.xxyx, s0, {f}.xyxx, {f}.zwzz
add {g}.z, {g}.z, -{g}.w
mul {h}.x, {h}.w, {f}.x
mad {h}.x, {g}.z, {f}.y, {h}.x
mul {h}.y, {h}.w, {f}.z
mad {h}.y, {g}.z, {f}.w, {h}.y
div {g}.x, {u}.y, {b}.z
mad {b}.xy, {h}.xyxx, {g}.xxxx, {b}.xyxx
add {h}.x, {a}.z, -{n}.z
add {h}.y, {a}.w, -{o}.z
mul {h}.z, {s}.z, {h}.x
mad {h}.z, -{t}.z, {h}.y, {h}.z
add {h}.z, {h}.z, {p}.z
mul {h}.w, {t}.z, {h}.x
mad {h}.w, {s}.z, {h}.y, {h}.w
add {h}.w, {h}.w, {r}.z
frc {q}.xy, {h}.zwzz
mul {f}.x, {s}.z, {d}.x
mad {f}.x, -{t}.z, {d}.y, {f}.x
mul {f}.y, {t}.z, {d}.x
mad {f}.y, {s}.z, {d}.y, {f}.y
mul {f}.z, {s}.z, {d}.z
mad {f}.z, -{t}.z, {d}.w, {f}.z
mul {f}.w, {t}.z, {d}.z
mad {f}.w, {s}.z, {d}.w, {f}.w
sample_d_indexable(texture2darray)(float,float,float,float) {g}.xy, {q}.xyzx, t116.xyxx, s0, {f}.xyxx, {f}.zwzz
mad {g}.xy, {g}.xyxx, l(3.011811, 3.011811, 0.000000, 0.000000), l(-1.511811, -1.511811, 0.000000, 0.000000)
mul {h}.x, {s}.z, {g}.x
mad {h}.x, {t}.z, {g}.y, {h}.x
mul {h}.y, {s}.z, {g}.y
mad {h}.y, -{t}.z, {g}.x, {h}.y
mad {j}.xy, {h}.xyxx, {u}.zzzz, {j}.xyxx
sample_d_indexable(texture2darray)(float,float,float,float) {g}.zw, {q}.xyzx, t117.xxxy, s0, {f}.xyxx, {f}.zwzz
mad {j}.z, {g}.z, {v}.z, {j}.z
max {h}.x, |{f}.x|, |{f}.y|
max {h}.y, |{f}.z|, |{f}.w|
max {h}.x, {h}.x, {h}.y
max {b}.z, {h}.x, l(0.000488281)
add {h}.x, {q}.x, {b}.z
mov {h}.y, {q}.y
frc {h}.xy, {h}.xyxx
mov {h}.z, {q}.z
sample_d_indexable(texture2darray)(float,float,float,float) {h}.w, {h}.xyzx, t117.xxxy, s0, {f}.xyxx, {f}.zwzz
add {h}.w, {h}.w, -{g}.w
mov {h}.x, {q}.x
add {h}.y, {q}.y, {b}.z
frc {h}.xy, {h}.xyxx
sample_d_indexable(texture2darray)(float,float,float,float) {g}.z, {h}.xyzx, t117.xxyx, s0, {f}.xyxx, {f}.zwzz
add {g}.z, {g}.z, -{g}.w
mul {h}.x, {h}.w, {f}.x
mad {h}.x, {g}.z, {f}.y, {h}.x
mul {h}.y, {h}.w, {f}.z
mad {h}.y, {g}.z, {f}.w, {h}.y
div {g}.x, {u}.z, {b}.z
mad {b}.xy, {h}.xyxx, {g}.xxxx, {b}.xyxx
div {j}.w, cb2[0].x, {k}.w
mul {j}.xy, {j}.xyxx, {j}.wwww
mul {i}.z, {j}.x, {d}.x
mad {i}.z, {j}.y, {d}.y, {i}.z
mul {i}.w, {j}.x, {d}.z
mad {i}.w, {j}.y, {d}.w, {i}.w
add {j}.z, {j}.z, l(-0.500000)
mul {k}.y, {j}.z, l(0.160000)
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(517, 0, 0, 0), t120.xyzw
eq {h}.x, {j}.x, l(0.000000)
movc {j}.x, {h}.x, l(0.500000), {j}.x
lt {h}.x, {j}.x, l(0.000000)
movc {j}.x, {h}.x, l(-10.000000), {j}.x
max {f}.x, {j}.x, l(0.020000)
min {f}.x, {f}.x, l(0.980000)
add {f}.y, -{f}.x, l(1.000000)
log {f}.x, {f}.x
log {f}.y, {f}.y
add {f}.x, {f}.x, -{f}.y
mad {f}.y, {f}.x, l(-0.002437424), l(-0.018739033)
mad {f}.y, {f}.y, {f}.x, l(-0.487268350)
mad {f}.y, {f}.y, {f}.x, l(0.264974522)
add {f}.x, {m}.x, -{f}.y
mad_sat {g}.w, {f}.x, l(0.400000), l(0.500000)
mad_sat {g}.w, {g}.w, l(-3.333333), l(3.333333)
mul {b}.xy, {b}.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000)
add {h}.xy, {i}.zwzz, -{b}.xyxx
mad {i}.zw, -{h}.xxxy, {g}.wwww, {i}.xxzw
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(514, 0, 0, 0), t120.xyzw
mov {q}.z, {j}.x
else
mov {k}.y, l(0.000000)
mov {i}.zw, l(0.000000, 0.000000, 0.000000, 0.000000)
endif
add {g}.xyz, v3.xyzx, -cb2[12].xyzx
dp3 {h}.w, cb2[12].xyzx, cb2[12].xyzx
rsq {h}.w, {h}.w
mul {h}.xyz, cb2[12].xyzx, {h}.wwww
mul {h}.xyz, {h}.xyzx, {h}.xyzx
mul {h}.xyz, {h}.xyzx, {h}.xyzx
mul {h}.xyz, {h}.xyzx, {h}.xyzx
dp3 {h}.w, {h}.xyzx, l(1.000000, 1.000000, 1.000000, 0.000000)
div {h}.xyz, {h}.xyzx, {h}.wwww
mul {a}.zw, {g}.yyyz, l(0.000000, 0.000000, 0.001538462, 0.001538462)
deriv_rtx_fine {d}.xy, v3.yzyy
deriv_rty_fine {d}.zw, v3.yyyz
mul {d}.xyzw, {d}.xyzw, l(0.001538462, 0.001538462, 0.001538462, 0.001538462)
frc {q}.xy, {a}.zwzz
sample_d_indexable(texture2darray)(float,float,float,float) {f}.xyzw, {q}.xyzx, t110.xyzw, s0, {d}.xyxx, {d}.zwzz
mul {f}.xyzw, {f}.xyzw, {h}.xxxx
mul {a}.zw, {g}.zzzx, l(0.000000, 0.000000, 0.001538462, 0.001538462)
deriv_rtx_fine {d}.xy, v3.zxzz
deriv_rty_fine {d}.zw, v3.zzzx
mul {d}.xyzw, {d}.xyzw, l(0.001538462, 0.001538462, 0.001538462, 0.001538462)
frc {q}.xy, {a}.zwzz
sample_d_indexable(texture2darray)(float,float,float,float) {b}.xyzw, {q}.xyzx, t110.xyzw, s0, {d}.xyxx, {d}.zwzz
mad {f}.xyzw, {b}.xyzw, {h}.yyyy, {f}.xyzw
mul {a}.zw, {g}.xxxy, l(0.000000, 0.000000, 0.001538462, 0.001538462)
deriv_rtx_fine {d}.xy, v3.xyxx
deriv_rty_fine {d}.zw, v3.xxxy
mul {d}.xyzw, {d}.xyzw, l(0.001538462, 0.001538462, 0.001538462, 0.001538462)
frc {q}.xy, {a}.zwzz
sample_d_indexable(texture2darray)(float,float,float,float) {b}.xyzw, {q}.xyzx, t110.xyzw, s0, {d}.xyxx, {d}.zwzz
mad {f}.xyzw, {b}.xyzw, {h}.zzzz, {f}.xyzw
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(514, 0, 0, 0), t120.xyzw
ge {b}.x, {j}.x, l(1.500000)
lt {b}.y, {j}.x, l(3.500000)
and {b}.x, {b}.x, {b}.y
if_nz {b}.x
deriv_rtx_fine {b}.xyz, v3.xyzx
deriv_rty_fine {d}.xyz, v3.xyzx
dp3 {b}.w, {b}.xyzx, {b}.xyzx
dp3 {d}.w, {d}.xyzx, {d}.xyzx
max {b}.w, {b}.w, {d}.w
sqrt {b}.w, {b}.w
lt {d}.w, l(0.000000), {c}.w
movc {d}.w, {d}.w, {c}.w, l(1.000000)
div {v}.z, {b}.w, {d}.w
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(517, 0, 0, 0), t120.xyzw
eq {b}.x, {j}.x, l(0.000000)
movc {j}.x, {b}.x, l(0.500000), {j}.x
lt {b}.x, {j}.x, l(0.000000)
movc {j}.x, {b}.x, l(-10.000000), {j}.x
mad {j}.y, {e}.y, l(-0.800000), l(0.800000)
mad_sat {j}.y, {e}.z, l(-0.800000), {j}.y
mad_sat {a}.z, {k}.y, l(32.786885), l(0.377049)
mad {j}.z, {a}.z, l(-2.000000), l(3.000000)
mul {a}.z, {a}.z, {a}.z
mul {a}.z, {a}.z, {j}.z
max {a}.z, {a}.z, {j}.y
max {b}.x, {j}.x, l(0.020000)
min {b}.x, {b}.x, l(0.980000)
add {b}.y, -{b}.x, l(1.000000)
log {b}.x, {b}.x
log {b}.y, {b}.y
add {b}.x, {b}.x, -{b}.y
mad {b}.y, {b}.x, l(-0.002437424), l(-0.018739033)
mad {b}.y, {b}.y, {b}.x, l(-0.487268350)
mad {b}.y, {b}.y, {b}.x, l(0.264974522)
add {b}.x, {m}.x, -{b}.y
mad_sat {a}.y, {b}.x, l(0.400000), l(0.500000)
add {b}.x, {a}.z, {a}.y
deriv_rtx_fine {b}.y, {b}.x
deriv_rty_fine {b}.z, {b}.x
add {b}.y, |{b}.y|, |{b}.z|
max {b}.y, {b}.y, l(0.000100)
rcp {b}.y, {b}.y
min {b}.y, {b}.y, l(6.000000)
add {b}.x, {b}.x, l(-0.983333)
mad_sat {b}.x, {b}.x, {b}.y, l(0.500000)
dp3 {b}.z, v3.xyzx, v3.xyzx
sqrt {b}.z, {b}.z
lt {b}.w, l(0.000000), {c}.w
movc {b}.w, {b}.w, {c}.w, l(1.000000)
div {b}.z, {b}.z, {b}.w
mad_sat {b}.y, {v}.z, l(20.000000000), l(-1.000000)
mad_sat {b}.z, {b}.z, l(0.001818182), l(-0.272727)
add {b}.w, {a}.y, -{b}.x
mad {b}.x, {b}.w, {b}.y, {b}.x
mad_sat {b}.w, {z}.w, l(1.600000), {j}.x
add {b}.w, {b}.w, -{b}.x
mad {b}.x, {b}.w, {b}.z, {b}.x
lt {b}.w, {j}.x, l(0.000000)
movc {m}.x, {b}.w, l(0.000000), {b}.x
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(514, 0, 0, 0), t120.xyzw
ge {b}.x, {j}.x, l(1.500000)
lt {b}.y, {j}.x, l(3.500000)
and {b}.x, {b}.x, {b}.y
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(520, 0, 0, 0), t120.xyzw
eq {b}.y, {j}.w, l(0.000000)
movc {j}.w, {b}.y, l(1.000000), {j}.w
max {j}.w, {j}.w, l(0.000000)
movc {m}.y, {b}.x, {j}.w, l(0.000000)
mad_sat {b}.x, {a}.y, l(-3.333333), l(3.333333)
mad {k}.y, -{k}.y, {b}.x, {k}.y
mov {k}.w, {a}.y
mad_sat {b}.w, {a}.y, l(2.222222), l(-1.000000)
mul {a}.y, {m}.x, {m}.y
min {a}.y, {a}.y, l(1.000000)
mad {b}.x, {a}.z, l(0.700000), l(0.300000)
mad {b}.y, {b}.w, l(-0.700000), l(1.000000)
mul {b}.x, {b}.x, {b}.y
mul {k}.y, {k}.y, {b}.x
mul {i}.zw, {i}.xyzw, {b}.xxxx
mad_sat {b}.x, {v}.z, l(-2.500000000), l(2.000000)
exp {w}.z, cb2[32].w
div {w}.w, cb2[0].x, {w}.z
mul {w}.xy, {w}.wwww, l(0.625000, 0.625000, 0.000000, 0.000000)
round_ne {w}.xy, {w}.xyxx
max {w}.xy, {w}.xyxx, l(1.000000, 1.000000, 0.000000, 0.000000)
mul {n}.xy, {w}.xyxx, {w}.zzzz
mov {o}.xy, v1.xyxx
mul {n}.xy, {o}.xyxx, {n}.xyxx
round_ni {o}.xy, {n}.xyxx
add {n}.xy, {n}.xyxx, -{o}.xyxx
div {u}.xy, {o}.xyxx, {w}.xyxx
round_ni {u}.xy, {u}.xyxx
mad {p}.xy, -{u}.xyxx, {w}.xyxx, {o}.xyxx
add {r}.xy, {o}.xyxx, l(1.000000, 1.000000, 0.000000, 0.000000)
div {u}.xy, {r}.xyxx, {w}.xyxx
round_ni {u}.xy, {u}.xyxx
mad {r}.xy, -{u}.xyxx, {w}.xyxx, {r}.xyxx
ftoi {p}.xy, {p}.xyxx
ftoi {r}.xy, {r}.xyxx
mov {s}.xz, {p}.xxxx
mov {s}.yw, {r}.xxxx
mov {t}.xy, {p}.yyyy
mov {t}.zw, {r}.yyyy
imul null, {s}.xyzw, {s}.xyzw, l(0x8da6b343, 0x8da6b343, 0x8da6b343, 0x8da6b343)
imul null, {t}.xyzw, {t}.xyzw, l(0xd8163841, 0xd8163841, 0xd8163841, 0xd8163841)
xor {s}.xyzw, {s}.xyzw, {t}.xyzw
iadd {s}.xyzw, {s}.xyzw, l(0x3b9ac6d1, 0x3b9ac6d1, 0x3b9ac6d1, 0x3b9ac6d1)
ushr {h}.xyzw, {s}.xyzw, l(16, 16, 16, 16)
xor {s}.xyzw, {s}.xyzw, {h}.xyzw
imul null, {s}.xyzw, {s}.xyzw, l(0x7feb352d, 0x7feb352d, 0x7feb352d, 0x7feb352d)
ushr {h}.xyzw, {s}.xyzw, l(15, 15, 15, 15)
xor {s}.xyzw, {s}.xyzw, {h}.xyzw
imul null, {s}.xyzw, {s}.xyzw, l(0x846ca68b, 0x846ca68b, 0x846ca68b, 0x846ca68b)
ushr {h}.xyzw, {s}.xyzw, l(16, 16, 16, 16)
xor {s}.xyzw, {s}.xyzw, {h}.xyzw
utof {s}.xyzw, {s}.xyzw
mul {s}.xyzw, {s}.xyzw, l(2.328306437E-10, 2.328306437E-10, 2.328306437E-10, 2.328306437E-10)
mad {u}.xy, {n}.xyxx, l(-2.000000, -2.000000, 0.000000, 0.000000), l(3.000000, 3.000000, 0.000000, 0.000000)
mul {t}.xy, {n}.xyxx, {n}.xyxx
mul {u}.xy, {u}.xyxx, {t}.xyxx
add {t}.xy, {s}.ywyy, -{s}.xzxx
mad {t}.xy, {t}.xyxx, {u}.xxxx, {s}.xzxx
add {t}.y, {t}.y, -{t}.x
mad {j}.x, {t}.y, {u}.y, {t}.x
add {j}.x, {j}.x, l(-0.500000)
mul {j}.x, {j}.x, {b}.x
mul {j}.x, {j}.x, l(0.250000)
add {j}.y, {j}.x, {k}.y
min {j}.z, {j}.y, l(0.000000)
mad_sat {v}.w, {j}.z, l(-20.000000), l(0.000000)
mul {v}.w, {v}.w, {b}.w
mul {v}.w, {v}.w, {a}.y
max {j}.w, {j}.y, l(0.000000)
mad {b}.x, {b}.w, l(-0.700000), l(1.000000)
mad {j}.z, {j}.z, {b}.x, {j}.w
mul {v}.x, {j}.z, {a}.y
mul {v}.x, {v}.x, l(2.000000)
mad {v}.y, {a}.y, l(-0.950000), l(1.000000)
mad {j}.w, {b}.w, l(-0.250000), l(1.000000)
mul {j}.w, {j}.w, {a}.y
deriv_rtx_fine {b}.y, {j}.x
deriv_rty_fine {b}.z, {j}.x
add {b}.y, {b}.y, {i}.z
add {b}.z, {b}.z, {i}.w
mul {b}.yz, {b}.xyzx, {j}.wwww
mul {i}.xy, {i}.xyxx, {v}.yyyy
add {i}.xy, {i}.xyxx, {b}.yzyy
else
mov {m}.xy, l(0.000000, 0.000000, 0.000000, 0.000000)
mov {v}.xyw, l(0.000000, 1.000000, 0.000000, 0.000000)
mov {k}.w, l(0.000000)
endif
add {e}.xyzw, {e}.xyzw, l(-0.500000, -0.500000, -0.500000, -0.500000)
mul {e}.yz, {e}.xyzx, {v}.yyyy
dp3 {a}.x, v3.xyzx, v3.xyzx
sqrt {a}.x, {a}.x
lt {a}.y, l(0.000000), {c}.w
movc {a}.y, {a}.y, {c}.w, l(1.000000)
div {a}.x, {a}.x, {a}.y
mad_sat {b}.xyzw, {a}.xxxx, l(-0.002000000, 0.000000000, -0.005882353, 0.000000000), l(1.500000, 1.000000, 1.470588, 1.000000)
mul {e}.xyzw, {e}.xyzw, {b}.xyzw
ld_indexable(texture1d)(float,float,float,float) {b}.xyzw, l(512, 0, 0, 0), t120.xyzw
eq {d}.xyzw, {b}.xyzw, l(0.000000, 0.000000, 0.000000, 0.000000)
movc {b}.xyzw, {d}.xyzw, l(1.000000, 1.000000, 1.000000, 1.000000), {b}.xyzw
mul {e}.xyzw, {e}.xyzw, {b}.xyzw
ld_indexable(texture1d)(float,float,float,float) {d}.xyzw, l(513, 0, 0, 0), t120.xyzw
eq {d}.y, {d}.x, l(0.000000)
movc {d}.x, {d}.y, l(1.000000), {d}.x
add {f}.x, {f}.x, l(-0.500000)
mul {f}.x, {f}.x, {d}.x
mad_sat {a}.y, {a}.x, l(-0.000100000), l(1.500000)
mul {f}.x, {f}.x, {a}.y
dp4 {a}.x, {e}.xyzw, l(0.500000, 0.900000, 0.300000, 0.200000)
mad {a}.x, {f}.x, l(0.600000), {a}.x
mul {k}.x, {k}.x, {k}.z
mul {k}.x, {k}.x, {v}.y
mad {a}.x, {k}.x, l(0.900000), {a}.x
add {a}.x, {a}.x, {v}.x
lt {a}.y, l(0.000000), {c}.x
movc {a}.y, {a}.y, {c}.x, l(1.000000)
mad {a}.x, {a}.x, {a}.y, l(1.000000)
max {a}.x, {a}.x, l(0.000000)
mul {src}.xyz, {src}.xyzx, {a}.xxxx
ld_indexable(texture1d)(float,float,float,float) {j}.xyzw, l(517, 0, 0, 0), t120.xyzw
eq {b}.xyz, {j}.yzwy, l(0.000000, 0.000000, 0.000000, 0.000000)
movc {j}.yzw, {b}.xxyz, l(1.000000, 1.000000, 0.700000, 1.000000), {j}.xyzw
min {b}.x, {a}.x, l(1.000000)
max {b}.x, {b}.x, l(0.020000)
log {b}.x, {b}.x
mul {b}.xyz, {b}.xxxx, l(0.200000, -0.050000, -0.400000, 0.000000)
mul {b}.xyz, {b}.xyzx, {j}.wwww
exp {b}.xyz, {b}.xyzx
add {b}.xyz, {b}.xyzx, l(-1.000000, -1.000000, -1.000000, 0.000000)
mad {b}.xyz, {b}.xyzx, {m}.yyyy, l(1.000000, 1.000000, 1.000000, 0.000000)
mul {src}.xyz, {src}.xyzx, {b}.xyzx
add {d}.xyz, -{src}.xyzx, l(1.000000, 1.000000, 1.000000, 0.000000)
mad {d}.xyz, {d}.xyzx, {j}.zzzz, {src}.xyzx
mad {e}.xyz, {j}.yyyy, l(-0.450000, -0.240000, 0.000000, 0.000000), l(1.000000, 1.000000, 1.000000, 0.000000)
mul {e}.xyz, {e}.xyzx, {src}.xyzx
mad_sat {b}.w, {k}.w, l(5.000000), l(-1.750000)
mad {b}.xyz, {b}.wwww, l(-0.840000, -0.760000, -0.680000, 0.000000), l(1.000000, 1.000000, 1.000000, 0.000000)
mul {e}.xyz, {e}.xyzx, {b}.xyzx
add {d}.xyz, {d}.xyzx, -{e}.xyzx
mad {e}.xyz, {d}.xyzx, {m}.xxxx, {e}.xyzx
add {e}.xyz, {e}.xyzx, -{src}.xyzx
mad {src}.xyz, {e}.xyzx, {m}.yyyy, {src}.xyzx
min {d}.xyz, {src}.xyzx, l(0.920000, 0.920000, 0.920000, 0.000000)
add {d}.xyz, {d}.xyzx, -{src}.xyzx
mul {b}.w, {m}.x, {m}.y
min {b}.w, {b}.w, l(1.000000)
mad {src}.xyz, {d}.xyzx, {b}.wwww, {src}.xyzx
mad {d}.xyz, {v}.wwww, l(-0.150000, -0.080000, 0.000000, 0.000000), l(1.000000, 1.000000, 1.000000, 0.000000)
mul {src}.xyz, {src}.xyzx, {d}.xyzx
mad {d}.xyz, {b}.wwww, l(0.090000, 0.210000, 0.370000, 0.000000), l(-0.450000, -0.450000, -0.450000, 0.000000)
mad {d}.xyz, {d}.xyzx, {z}.zzzz, l(1.000000, 1.000000, 1.000000, 0.000000)
mul {src}.xyz, {src}.xyzx, {d}.xyzx
lt {a}.x, l(0.000000), {c}.z
movc {a}.x, {a}.x, {c}.z, l(1.000000)
mul {a}.yz, {i}.xxyx, {a}.xxxx
deriv_rtx_fine {b}.xyz, v3.xyzx
deriv_rty_fine {d}.xyz, v3.xyzx
dp3 {j}.w, {g}.xyzx, {g}.xyzx
rsq {j}.w, {j}.w
mul {j}.xyz, {g}.xyzx, {j}.wwww
dp3 {k}.y, {j}.xyzx, {b}.xyzx
dp3 {k}.z, {j}.xyzx, {d}.xyzx
eq {j}.w, {c}.y, l(5.000000)
movc {a}.yz, {j}.wwww, -{k}.yyzy, {a}.yyzy
dp3 {a}.w, {normal}.xyzx, {normal}.xyzx
rsq {a}.w, {a}.w
mul {f}.xyz, {normal}.xyzx, {a}.wwww
mul {h}.xyz, {d}.zxyz, {f}.yzxy
mad {h}.xyz, {d}.yzxy, {f}.zxyz, -{h}.xyzx
mul {e}.xyz, {f}.zxyz, {b}.yzxy
mad {e}.xyz, {f}.yzxy, {b}.zxyz, -{e}.xyzx
dp3 {a}.w, {b}.xyzx, {h}.xyzx
mul {h}.xyz, {h}.xyzx, {a}.yyyy
mad {h}.xyz, {e}.xyzx, {a}.zzzz, {h}.xyzx
lt {b}.x, {a}.w, l(0.000000)
movc {h}.xyz, {b}.xxxx, -{h}.xyzx, {h}.xyzx
mad {h}.xyz, {f}.xyzx, |{a}.wwww|, -{h}.xyzx
dp3 {a}.w, {h}.xyzx, {h}.xyzx
lt {b}.y, l(0.000000), {a}.w
rsq {a}.w, {a}.w
mul {h}.xyz, {h}.xyzx, {a}.wwww
movc {normal}.xyz, {b}.yyyy, {h}.xyzx, {normal}.xyzx
exp {a}.x, cb2[32].w
div {a}.y, cb2[0].x, {a}.x
mul {a}.y, {a}.y, l(0.125000)
round_ne {a}.y, {a}.y
max {a}.y, {a}.y, l(1.000000)
mul {a}.x, {a}.y, {a}.x
mul {a}.zw, v1.xxxy, {a}.xxxx
frc {a}.zw, {a}.zzzw
mul {a}.y, cb2[32].w, l(0.083333)
mul {g}.xyz, {g}.xzyx, l(0.012500, 0.012500, 0.012500, 0.000000)
frc {g}.xyz, {g}.xyzx
lt {d}.xyzw, l(0.500000, 1.500000, 2.500000, 3.500000), {c}.yyyy
lt {j}.w, {c}.y, l(4.500000)
and {d}.xyzw, {d}.xyzw, {j}.wwww
movc {src}.xyz, {d}.xxxx, {a}.zwyz, {src}.xyzx
movc {src}.xyz, {d}.yyyy, {g}.xyzx, {src}.xyzx
mad {g}.xyz, {normal}.xyzx, l(0.500000, 0.500000, 0.500000, 0.000000), l(0.500000, 0.500000, 0.500000, 0.000000)
movc {src}.xyz, {d}.zzzz, {g}.xyzx, {src}.xyzx
add {g}.xyz, {normal}.xyzx, -{f}.xyzx
mad {g}.xyz, {g}.xyzx, l(4.000000, 4.000000, 4.000000, 0.000000), l(0.500000, 0.500000, 0.500000, 0.000000)
movc {src}.xyz, {d}.wwww, {g}.xyzx, {src}.xyzx
eq {j}.w, {c}.y, l(6.000000)
mov {g}.xy, {m}.xxxx
add {g}.z, -{m}.x, l(1.000000)
mul {g}.xyz, {g}.xyzx, l(1.000000, 0.200000, 1.000000, 0.000000)
movc {src}.xyz, {j}.wwww, {g}.xyzx, {src}.xyzx
eq {j}.w, {c}.y, l(9.000000)
movc {src}.xyz, {j}.wwww, {m}.zzzz, {src}.xyzx
eq {j}.w, {c}.y, l(10.000000)
movc {src}.xyz, {j}.wwww, {z}.zzzz, {src}.xyzx
else
mov {m}.x, l(0.000000)
endif
