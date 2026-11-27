// EDCAS star lens (edcas_star_lens.hlsl, star_lens.py): temps 11; do not edit, run star_lens.py
mad {r0}.xyz, v2.xyzx, cb0[0].xyzx, cb0[1].xyzx
dp3 {r0}.w, {r0}.xyzx, {r0}.xyzx
rsq {r0}.w, {r0}.w
mul {r0}.xyz, {r0}.wwww, {r0}.xyzx
lt {r0}.w, l(0x3f000000), v1.x
lt {r1}.x, l(0x00000000), cb1[77].x
if_nz {r1}.x
mad {r1}.xyz, cb1[74].xyzx, cb0[0].xyzx, cb0[1].xyzx
dp3 {r1}.w, {r1}.xyzx, {r1}.xyzx
rsq {r1}.w, {r1}.w
mul {r1}.xyz, {r1}.wwww, {r1}.xyzx
dp3 {r1}.w, {r0}.xyzx, {r1}.xyzx
mad {r2}.xyz, -{r1}.xyzx, {r1}.wwww, {r0}.xyzx
dp3 {r2}.w, {r2}.xyzx, {r2}.xyzx
sqrt {r2}.w, {r2}.w
ge {r3}.x, {r2}.w, l(0x33d6bf95)
if_nz {r3}.x
max {r1}.w, {r1}.w, l(0xbf800000)
min {r1}.w, {r1}.w, l(0x3f800000)
add {r3}.x, -|{r1}.w|, l(0x3f800000)
sqrt {r3}.x, {r3}.x
mad {r3}.y, |{r1}.w|, l(0xbc996e30), l(0x3d981627)
mad {r3}.y, {r3}.y, |{r1}.w|, l(0xbe593484)
mad {r3}.y, {r3}.y, |{r1}.w|, l(0x3fc90da4)
mul {r3}.z, {r3}.x, {r3}.y
mad {r3}.z, {r3}.z, l(0xc0000000), l(0x40490fdb)
lt {r1}.w, {r1}.w, -{r1}.w
and {r1}.w, {r1}.w, {r3}.z
mad {r1}.w, {r3}.y, {r3}.x, {r1}.w
add {r3}.x, cb1[77].x, cb1[77].x
mul {r3}.y, {r1}.w, {r1}.w
div {r3}.x, {r3}.y, {r3}.x
add {r3}.yz, {r3}.xxxx, l(0x00000000, 0x40000000, 0x40800000, 0x00000000)
mul {r3}.x, {r3}.z, {r3}.x
sqrt {r3}.x, {r3}.x
add {r3}.x, {r3}.x, {r3}.x
div {r3}.x, {r3}.y, {r3}.x
add {r3}.x, {r3}.x, l(0xbf000000)
max {r3}.x, {r3}.x, l(0x0da24260)
log {r3}.x, {r3}.x
mad {r3}.x, -{r3}.x, l(0x3f40a8c1), r1.w
add {r3}.y, cb2[1].x, l(0x3f000000)
lt {r3}.x, {r3}.y, {r3}.x
and {r3}.x, {r0}.w, {r3}.x
if_z {r3}.x
div {r2}.xyz, {r2}.xyzx, {r2}.wwww
movc {r2}.w, {r0}.w, -{r1}.w, {r1}.w
mul {r3}.xy, cb1[77].xxxx, l(0x402646e1, 0x41000000, 0x00000000, 0x00000000)
add {r3}.z, -cb1[77].x, l(0x3f800000)
sqrt {r3}.z, {r3}.z
mul {r3}.x, {r3}.z, {r3}.x
min {r3}.x, {r3}.x, l(0x3f800000)
add {r3}.w, -|{r3}.x|, l(0x3f800000)
sqrt {r3}.w, {r3}.w
mad {r4}.x, |{r3}.x|, l(0xbc996e30), l(0x3d981627)
mad {r4}.x, {r4}.x, |{r3}.x|, l(0xbe593484)
mad {r4}.x, {r4}.x, |{r3}.x|, l(0x3fc90da4)
mul {r4}.y, {r3}.w, {r4}.x
mad {r4}.y, {r4}.y, l(0xc0000000), l(0x40490fdb)
lt {r3}.x, {r3}.x, -{r3}.x
and {r3}.x, {r3}.x, {r4}.y
mad {r3}.x, {r4}.x, {r3}.w, {r3}.x
add {r3}.x, -{r3}.x, l(0x3fc90fdb)
mad {r3}.y, {r1}.w, {r1}.w, {r3}.y
sqrt {r3}.y, {r3}.y
add {r3}.y, {r2}.w, {r3}.y
mul {r3}.xy, {r3}.xyxx, l(0x3f800054, 0x3f000000, 0x00000000, 0x00000000)
max {r3}.y, {r3}.x, {r3}.y
min {r3}.y, {r3}.y, l(0x40490fdb)
mul {r3}.w, {r3}.z, cb1[77].x
mov {r4}.x, {r3}.x
mov {r5}.x, {r3}.y
mov {r4}.yzw, l(0x00000000,0x40490fdb,0x3f800000,0x00000000)
loop
ige {r5}.w, {r4}.w, l(0x00000008)
breakc_nz {r5}.w
mul {r5}.w, {r5}.x, l(0x3a83126f)
max {r5}.w, {r5}.w, l(0x358637bd)
sincos {r6}.x, {r7}.x, {r5}.x
add {r6}.y, -|{r7}.x|, l(0x3f800000)
sqrt {r6}.y, {r6}.y
mad {r6}.z, |{r7}.x|, l(0xbc996e30), l(0x3d981627)
mad {r6}.z, {r6}.z, |{r7}.x|, l(0xbe593484)
mad {r6}.z, {r6}.z, |{r7}.x|, l(0x3fc90da4)
mul {r6}.w, {r6}.y, {r6}.z
mad {r6}.w, {r6}.w, l(0xc0000000), l(0x40490fdb)
lt {r7}.y, {r7}.x, -{r7}.x
and {r6}.w, {r6}.w, {r7}.y
mad {r6}.y, {r6}.z, {r6}.y, {r6}.w
div {r6}.z, {r6}.x, {r3}.w
lt {r6}.w, l(0x00000000), {r7}.x
ge {r7}.y, l(0x402646ec), {r6}.z
and {r6}.w, {r6}.w, {r7}.y
not {r7}.y, {r6}.w
if_z {r6}.w
lt {r6}.w, l(0x3f000000), {r7}.x
lt {r7}.z, {r6}.z, l(0x42700000)
and {r6}.w, {r6}.w, {r7}.z
if_nz {r6}.w
div {r6}.w, {r6}.x, {r3}.z
min {r6}.w, {r6}.w, l(0x3f800000)
mad {r7}.z, -{r6}.w, {r6}.w, l(0x3f800000)
add {r7}.w, -|{r6}.w|, l(0x3f800000)
sqrt {r7}.zw, {r7}.zzzw
mad {r8}.x, |{r6}.w|, l(0xbc996e30), l(0x3d981627)
mad {r8}.x, {r8}.x, |{r6}.w|, l(0xbe593484)
mad {r8}.x, {r8}.x, |{r6}.w|, l(0x3fc90da4)
mul {r8}.y, {r7}.w, {r8}.x
mad {r8}.y, {r8}.y, l(0xc0000000), l(0x40490fdb)
lt {r6}.w, {r6}.w, -{r6}.w
and {r6}.w, {r6}.w, {r8}.y
mad {r6}.w, {r8}.x, {r7}.w, {r6}.w
add {r6}.w, -{r6}.w, l(0x3fc90fdb)
div {r7}.w, l(0x3f800000, 0x3f800000, 0x3f800000, 0x3f800000), {r7}.z
add {r7}.z, {r7}.w, {r7}.z
add {r7}.z, {r7}.z, l(0xc0000000)
add {r7}.w, {r6}.z, {r6}.z
div {r7}.z, {r7}.z, {r7}.w
add {r6}.w, {r6}.w, -{r7}.z
mul {r7}.z, {r6}.z, l(0x3f93cd3a)
div {r7}.w, l(0xc02646e1), {r6}.z
add {r8}.x, -|{r7}.w|, l(0x3f800000)
sqrt {r8}.x, {r8}.x
mad {r8}.y, |{r7}.w|, l(0xbc996e30), l(0x3d981627)
mad {r8}.y, {r8}.y, |{r7}.w|, l(0xbe593484)
mad {r8}.y, {r8}.y, |{r7}.w|, l(0x3fc90da4)
mul {r8}.z, {r8}.x, {r8}.y
mad {r8}.z, {r8}.z, l(0xc0000000), l(0x40490fdb)
lt {r7}.w, {r7}.w, -{r7}.w
and {r7}.w, {r7}.w, {r8}.z
mad {r7}.w, {r8}.y, {r8}.x, {r7}.w
mul {r7}.w, {r7}.w, l(0x3eaaaaab)
sincos null, {r7}.w, {r7}.w
mul {r8}.x, {r7}.w, {r7}.z
mad {r8}.yz, {r7}.zzzz, {r7}.wwww, l(0x00000000, 0xbf800000, 0x40400000, 0x00000000)
mul {r8}.y, {r8}.z, {r8}.y
sqrt {r8}.y, {r8}.y
mad {r8}.z, {r8}.x, l(0x40000000), l(0xc0400000)
mad {r7}.z, {r7}.z, {r7}.w, {r8}.y
div {r7}.z, {r8}.z, {r7}.z
add {r7}.zw, {r7}.zzzz, l(0x00000000, 0x00000000, 0x40400000, 0x3f800000)
add {r8}.z, {r8}.y, {r8}.y
div {r8}.z, {r7}.z, {r8}.z
min {r8}.z, {r8}.z, l(0x3f7ffffe)
div {r7}.z, {r7}.w, {r7}.z
add {r7}.w, -{r8}.z, l(0x3f800000)
sqrt {r8}.w, {r7}.w
mul {r9}.x, {r8}.w, l(0x3e800000)
add {r7}.w, {r7}.w, {r8}.w
mul {r9}.y, {r7}.w, l(0x3e800000)
add {r9}.z, {r8}.w, l(0x3f800000)
mul {r9}.w, {r9}.z, l(0x3e800000)
sqrt {r9}.xyw, {r9}.xyxw
mul {r10}.x, {r9}.w, {r9}.y
mad {r9}.y, {r9}.x, {r9}.y, {r10}.x
mad {r9}.x, {r9}.w, {r9}.x, {r9}.y
mad {r8}.w, {r8}.w, l(0x3e800000), {r9}.x
mul {r9}.y, {r8}.w, l(0x3e800000)
mad {r7}.w, {r7}.w, l(0x3e800000), {r9}.x
mul {r9}.w, {r7}.w, l(0x3e800000)
mad {r9}.x, {r9}.z, l(0x3e800000), {r9}.x
mul {r9}.z, {r9}.x, l(0x3e800000)
sqrt {r9}.yzw, {r9}.yyzw
mul {r10}.x, {r9}.z, {r9}.w
mad {r9}.w, {r9}.y, {r9}.w, {r10}.x
mad {r9}.y, {r9}.z, {r9}.y, {r9}.w
mad {r8}.w, {r8}.w, l(0x3e800000), {r9}.y
mul {r9}.z, {r8}.w, l(0x3e800000)
mad {r7}.w, {r7}.w, l(0x3e800000), {r9}.y
mul {r7}.w, {r7}.w, l(0x3e800000)
mad {r9}.x, {r9}.x, l(0x3e800000), {r9}.y
mad {r8}.w, {r8}.w, l(0x3e800000), {r7}.w
mad {r8}.w, {r9}.x, l(0x3e800000), {r8}.w
mul {r8}.w, {r8}.w, l(0x3eaaaaab)
div {r9}.x, {r9}.z, {r8}.w
add {r9}.x, -{r9}.x, l(0x3f800000)
div {r7}.w, {r7}.w, {r8}.w
add {r7}.w, -{r7}.w, l(0x3f800000)
add {r9}.y, -{r7}.w, -{r9}.x
mul {r7}.w, {r7}.w, {r9}.x
mad {r9}.x, -{r9}.y, {r9}.y, {r7}.w
mul {r7}.w, {r9}.y, {r7}.w
mad {r9}.y, -{r9}.x, l(0x3dcccccd), l(0x3f800000)
mad {r9}.y, {r7}.w, l(0x3d924925), {r9}.y
mul {r9}.z, {r9}.x, {r9}.x
mad {r9}.y, {r9}.z, l(0x3d2aaaab), {r9}.y
mul {r7}.w, {r9}.x, {r7}.w
mad {r7}.w, -{r7}.w, l(0x3d8ba2e9), {r9}.y
sqrt {r8}.w, {r8}.w
div {r7}.w, {r7}.w, {r8}.w
sqrt {r8}.w, {r7}.z
add {r9}.x, -{r7}.z, l(0x3f800000)
mad {r7}.z, -{r8}.z, {r7}.z, l(0x3f800000)
sqrt {r8}.z, {r9}.x
sqrt {r9}.y, {r7}.z
mad {r9}.y, {r8}.z, {r9}.y, {r9}.y
add {r8}.z, {r8}.z, {r9}.y
add {r9}.x, {r8}.z, {r9}.x
mul {r9}.y, {r9}.x, l(0x3e800000)
add {r7}.z, {r7}.z, {r8}.z
mul {r9}.z, {r7}.z, l(0x3e800000)
add {r8}.z, {r8}.z, l(0x3f800000)
mul {r9}.w, {r8}.z, l(0x3e800000)
sqrt {r9}.yzw, {r9}.yyzw
mul {r10}.x, {r9}.w, {r9}.z
mad {r9}.z, {r9}.y, {r9}.z, {r10}.x
mad {r9}.y, {r9}.w, {r9}.y, {r9}.z
mad {r9}.x, {r9}.x, l(0x3e800000), {r9}.y
mul {r9}.z, {r9}.x, l(0x3e800000)
mad {r7}.z, {r7}.z, l(0x3e800000), {r9}.y
mul {r9}.w, {r7}.z, l(0x3e800000)
mad {r8}.z, {r8}.z, l(0x3e800000), {r9}.y
mul {r9}.y, {r8}.z, l(0x3e800000)
sqrt {r9}.yzw, {r9}.yyzw
mul {r10}.x, {r9}.y, {r9}.w
mad {r9}.w, {r9}.z, {r9}.w, {r10}.x
mad {r9}.y, {r9}.y, {r9}.z, {r9}.w
mad {r9}.x, {r9}.x, l(0x3e800000), {r9}.y
mul {r9}.z, {r9}.x, l(0x3e800000)
mad {r7}.z, {r7}.z, l(0x3e800000), {r9}.y
mul {r7}.z, {r7}.z, l(0x3e800000)
mad {r8}.z, {r8}.z, l(0x3e800000), {r9}.y
mad {r9}.x, {r9}.x, l(0x3e800000), {r7}.z
mad {r8}.z, {r8}.z, l(0x3e800000), {r9}.x
mul {r8}.z, {r8}.z, l(0x3eaaaaab)
div {r9}.x, {r9}.z, {r8}.z
add {r9}.x, -{r9}.x, l(0x3f800000)
div {r7}.z, {r7}.z, {r8}.z
add {r7}.z, -{r7}.z, l(0x3f800000)
add {r9}.y, -{r7}.z, -{r9}.x
mul {r7}.z, {r7}.z, {r9}.x
mad {r9}.x, -{r9}.y, {r9}.y, {r7}.z
mul {r7}.z, {r9}.y, {r7}.z
mad {r9}.y, -{r9}.x, l(0x3dcccccd), l(0x3f800000)
mad {r9}.y, {r7}.z, l(0x3d924925), {r9}.y
mul {r9}.z, {r9}.x, {r9}.x
mad {r9}.y, {r9}.z, l(0x3d2aaaab), {r9}.y
mul {r7}.z, {r9}.x, {r7}.z
mad {r7}.z, -{r7}.z, l(0x3d8ba2e9), {r9}.y
sqrt {r8}.z, {r8}.z
div {r7}.z, {r7}.z, {r8}.z
div {r8}.x, {r8}.x, {r8}.y
sqrt {r8}.x, {r8}.x
mul {r8}.x, {r8}.x, l(0x40800000)
mad {r7}.z, -{r8}.w, {r7}.z, {r7}.w
mad {r7}.z, {r8}.x, {r7}.z, l(0xc0490fdb)
add {r6}.y, {r6}.w, -{r7}.z
else
lt {r6}.w, l(0x00000000), {r6}.x
add {r7}.z, {r7}.x, l(0x3f800000)
div {r7}.z, {r7}.z, {r6}.z
add {r7}.z, {r6}.y, -{r7}.z
add {r7}.w, -{r6}.y, l(0x40490fdb)
mad {r6}.x, {r6}.x, {r7}.x, {r7}.w
mul {r6}.x, {r6}.x, l(0x3f700000)
mul {r6}.z, {r6}.z, {r6}.z
div {r6}.x, {r6}.x, {r6}.z
add {r6}.x, -{r6}.x, {r7}.z
movc {r6}.y, {r6}.w, {r6}.x, {r6}.y
endif
mov {r7}.y, l(0xffffffff)
endif
movc {r6}.x, {r7}.y, {r6}.y, l(0xce6e6b28)
add {r6}.y, {r5}.w, {r5}.x
sincos {r7}.x, {r8}.x, {r6}.y
add {r6}.y, -|{r8}.x|, l(0x3f800000)
sqrt {r6}.y, {r6}.y
mad {r6}.z, |{r8}.x|, l(0xbc996e30), l(0x3d981627)
mad {r6}.z, {r6}.z, |{r8}.x|, l(0xbe593484)
mad {r6}.z, {r6}.z, |{r8}.x|, l(0x3fc90da4)
mul {r6}.w, {r6}.y, {r6}.z
mad {r6}.w, {r6}.w, l(0xc0000000), l(0x40490fdb)
lt {r7}.y, {r8}.x, -{r8}.x
and {r6}.w, {r6}.w, {r7}.y
mad {r6}.y, {r6}.z, {r6}.y, {r6}.w
div {r6}.z, {r7}.x, {r3}.w
lt {r6}.w, l(0x00000000), {r8}.x
ge {r7}.y, l(0x402646ec), {r6}.z
and {r6}.w, {r6}.w, {r7}.y
not {r7}.y, {r6}.w
if_z {r6}.w
lt {r6}.w, l(0x3f000000), {r8}.x
lt {r7}.z, {r6}.z, l(0x42700000)
and {r6}.w, {r6}.w, {r7}.z
if_nz {r6}.w
div {r6}.w, {r7}.x, {r3}.z
min {r6}.w, {r6}.w, l(0x3f800000)
mad {r7}.z, -{r6}.w, {r6}.w, l(0x3f800000)
add {r7}.w, -|{r6}.w|, l(0x3f800000)
sqrt {r7}.zw, {r7}.zzzw
mad {r8}.y, |{r6}.w|, l(0xbc996e30), l(0x3d981627)
mad {r8}.y, {r8}.y, |{r6}.w|, l(0xbe593484)
mad {r8}.y, {r8}.y, |{r6}.w|, l(0x3fc90da4)
mul {r8}.z, {r7}.w, {r8}.y
mad {r8}.z, {r8}.z, l(0xc0000000), l(0x40490fdb)
lt {r6}.w, {r6}.w, -{r6}.w
and {r6}.w, {r6}.w, {r8}.z
mad {r6}.w, {r8}.y, {r7}.w, {r6}.w
add {r6}.w, -{r6}.w, l(0x3fc90fdb)
div {r7}.w, l(0x3f800000, 0x3f800000, 0x3f800000, 0x3f800000), {r7}.z
add {r7}.z, {r7}.w, {r7}.z
add {r7}.z, {r7}.z, l(0xc0000000)
add {r7}.w, {r6}.z, {r6}.z
div {r7}.z, {r7}.z, {r7}.w
add {r6}.w, {r6}.w, -{r7}.z
mul {r7}.z, {r6}.z, l(0x3f93cd3a)
div {r7}.w, l(0xc02646e1), {r6}.z
add {r8}.y, -|{r7}.w|, l(0x3f800000)
sqrt {r8}.y, {r8}.y
mad {r8}.z, |{r7}.w|, l(0xbc996e30), l(0x3d981627)
mad {r8}.z, {r8}.z, |{r7}.w|, l(0xbe593484)
mad {r8}.z, {r8}.z, |{r7}.w|, l(0x3fc90da4)
mul {r8}.w, {r8}.y, {r8}.z
mad {r8}.w, {r8}.w, l(0xc0000000), l(0x40490fdb)
lt {r7}.w, {r7}.w, -{r7}.w
and {r7}.w, {r7}.w, {r8}.w
mad {r7}.w, {r8}.z, {r8}.y, {r7}.w
mul {r7}.w, {r7}.w, l(0x3eaaaaab)
sincos null, {r7}.w, {r7}.w
mul {r8}.y, {r7}.w, {r7}.z
mad {r8}.zw, {r7}.zzzz, {r7}.wwww, l(0x00000000, 0x00000000, 0xbf800000, 0x40400000)
mul {r8}.z, {r8}.w, {r8}.z
sqrt {r8}.z, {r8}.z
mad {r8}.w, {r8}.y, l(0x40000000), l(0xc0400000)
mad {r7}.z, {r7}.z, {r7}.w, {r8}.z
div {r7}.z, {r8}.w, {r7}.z
add {r7}.zw, {r7}.zzzz, l(0x00000000, 0x00000000, 0x40400000, 0x3f800000)
add {r8}.w, {r8}.z, {r8}.z
div {r8}.w, {r7}.z, {r8}.w
min {r8}.w, {r8}.w, l(0x3f7ffffe)
div {r7}.z, {r7}.w, {r7}.z
add {r7}.w, -{r8}.w, l(0x3f800000)
sqrt {r9}.x, {r7}.w
mul {r9}.y, {r9}.x, l(0x3e800000)
add {r7}.w, {r7}.w, {r9}.x
mul {r9}.z, {r7}.w, l(0x3e800000)
add {r9}.w, {r9}.x, l(0x3f800000)
mul {r10}.x, {r9}.w, l(0x3e800000)
sqrt {r9}.yz, {r9}.yyzy
sqrt {r10}.x, {r10}.x
mul {r10}.y, {r9}.z, {r10}.x
mad {r9}.z, {r9}.y, {r9}.z, {r10}.y
mad {r9}.y, {r10}.x, {r9}.y, {r9}.z
mad {r9}.x, {r9}.x, l(0x3e800000), {r9}.y
mad {r7}.w, {r7}.w, l(0x3e800000), {r9}.y
mul {r10}.x, {r7}.w, l(0x3e800000)
mad {r9}.y, {r9}.w, l(0x3e800000), {r9}.y
mul {r9}.zw, {r9}.xxxy, l(0x00000000, 0x00000000, 0x3e800000, 0x3e800000)
sqrt {r10}.x, {r10}.x
sqrt {r9}.zw, {r9}.zzzw
mul {r10}.y, {r9}.w, {r10}.x
mad {r10}.x, {r9}.z, {r10}.x, {r10}.y
mad {r9}.z, {r9}.w, {r9}.z, {r10}.x
mad {r9}.x, {r9}.x, l(0x3e800000), {r9}.z
mul {r9}.w, {r9}.x, l(0x3e800000)
mad {r7}.w, {r7}.w, l(0x3e800000), {r9}.z
mul {r7}.w, {r7}.w, l(0x3e800000)
mad {r9}.y, {r9}.y, l(0x3e800000), {r9}.z
mad {r9}.x, {r9}.x, l(0x3e800000), {r7}.w
mad {r9}.x, {r9}.y, l(0x3e800000), {r9}.x
mul {r9}.x, {r9}.x, l(0x3eaaaaab)
div {r9}.y, {r9}.w, {r9}.x
add {r9}.y, -{r9}.y, l(0x3f800000)
div {r7}.w, {r7}.w, {r9}.x
add {r7}.w, -{r7}.w, l(0x3f800000)
add {r9}.z, -{r7}.w, -{r9}.y
mul {r7}.w, {r7}.w, {r9}.y
mad {r9}.y, -{r9}.z, {r9}.z, {r7}.w
mul {r7}.w, {r9}.z, {r7}.w
mad {r9}.z, -{r9}.y, l(0x3dcccccd), l(0x3f800000)
mad {r9}.z, {r7}.w, l(0x3d924925), {r9}.z
mul {r9}.w, {r9}.y, {r9}.y
mad {r9}.z, {r9}.w, l(0x3d2aaaab), {r9}.z
mul {r7}.w, {r9}.y, {r7}.w
mad {r7}.w, -{r7}.w, l(0x3d8ba2e9), {r9}.z
sqrt {r9}.x, {r9}.x
div {r7}.w, {r7}.w, {r9}.x
sqrt {r9}.x, {r7}.z
add {r9}.y, -{r7}.z, l(0x3f800000)
mad {r7}.z, -{r8}.w, {r7}.z, l(0x3f800000)
sqrt {r8}.w, {r9}.y
sqrt {r9}.z, {r7}.z
mad {r9}.z, {r8}.w, {r9}.z, {r9}.z
add {r8}.w, {r8}.w, {r9}.z
add {r9}.y, {r8}.w, {r9}.y
mul {r9}.z, {r9}.y, l(0x3e800000)
add {r7}.z, {r7}.z, {r8}.w
mul {r9}.w, {r7}.z, l(0x3e800000)
add {r8}.w, {r8}.w, l(0x3f800000)
mul {r10}.x, {r8}.w, l(0x3e800000)
sqrt {r9}.zw, {r9}.zzzw
sqrt {r10}.x, {r10}.x
mul {r10}.y, {r9}.w, {r10}.x
mad {r9}.w, {r9}.z, {r9}.w, {r10}.y
mad {r9}.z, {r10}.x, {r9}.z, {r9}.w
mad {r9}.y, {r9}.y, l(0x3e800000), {r9}.z
mul {r9}.w, {r9}.y, l(0x3e800000)
mad {r7}.z, {r7}.z, l(0x3e800000), {r9}.z
mul {r10}.x, {r7}.z, l(0x3e800000)
mad {r8}.w, {r8}.w, l(0x3e800000), {r9}.z
mul {r9}.z, {r8}.w, l(0x3e800000)
sqrt {r10}.x, {r10}.x
sqrt {r9}.zw, {r9}.zzzw
mul {r10}.y, {r9}.z, {r10}.x
mad {r10}.x, {r9}.w, {r10}.x, {r10}.y
mad {r9}.z, {r9}.z, {r9}.w, {r10}.x
mad {r9}.y, {r9}.y, l(0x3e800000), {r9}.z
mul {r9}.w, {r9}.y, l(0x3e800000)
mad {r7}.z, {r7}.z, l(0x3e800000), {r9}.z
mul {r7}.z, {r7}.z, l(0x3e800000)
mad {r8}.w, {r8}.w, l(0x3e800000), {r9}.z
mad {r9}.y, {r9}.y, l(0x3e800000), {r7}.z
mad {r8}.w, {r8}.w, l(0x3e800000), {r9}.y
mul {r8}.w, {r8}.w, l(0x3eaaaaab)
div {r9}.y, {r9}.w, {r8}.w
add {r9}.y, -{r9}.y, l(0x3f800000)
div {r7}.z, {r7}.z, {r8}.w
add {r7}.z, -{r7}.z, l(0x3f800000)
add {r9}.z, -{r7}.z, -{r9}.y
mul {r7}.z, {r7}.z, {r9}.y
mad {r9}.y, -{r9}.z, {r9}.z, {r7}.z
mul {r7}.z, {r9}.z, {r7}.z
mad {r9}.z, -{r9}.y, l(0x3dcccccd), l(0x3f800000)
mad {r9}.z, {r7}.z, l(0x3d924925), {r9}.z
mul {r9}.w, {r9}.y, {r9}.y
mad {r9}.z, {r9}.w, l(0x3d2aaaab), {r9}.z
mul {r7}.z, {r9}.y, {r7}.z
mad {r7}.z, -{r7}.z, l(0x3d8ba2e9), {r9}.z
sqrt {r8}.w, {r8}.w
div {r7}.z, {r7}.z, {r8}.w
div {r8}.y, {r8}.y, {r8}.z
sqrt {r8}.y, {r8}.y
mul {r8}.y, {r8}.y, l(0x40800000)
mad {r7}.z, -{r9}.x, {r7}.z, {r7}.w
mad {r7}.z, {r8}.y, {r7}.z, l(0xc0490fdb)
add {r6}.y, {r6}.w, -{r7}.z
else
lt {r6}.w, l(0x00000000), {r7}.x
add {r7}.z, {r8}.x, l(0x3f800000)
div {r7}.z, {r7}.z, {r6}.z
add {r7}.z, {r6}.y, -{r7}.z
add {r7}.w, -{r6}.y, l(0x40490fdb)
mad {r7}.x, {r7}.x, {r8}.x, {r7}.w
mul {r7}.x, {r7}.x, l(0x3f700000)
mul {r6}.z, {r6}.z, {r6}.z
div {r6}.z, {r7}.x, {r6}.z
add {r6}.z, -{r6}.z, {r7}.z
movc {r6}.y, {r6}.w, {r6}.z, {r6}.y
endif
mov {r7}.y, l(0xffffffff)
endif
movc {r6}.y, {r7}.y, {r6}.y, l(0xce6e6b28)
add {r6}.xy, -{r2}.wwww, {r6}.xyxx
add {r6}.y, -{r6}.x, {r6}.y
div {r5}.w, {r6}.y, {r5}.w
max {r6}.y, {r5}.w, l(0x3089705f)
mul {r6}.y, {r6}.y, l(0x358637bd)
ge {r6}.y, {r6}.y, |{r6}.x|
if_nz {r6}.y
mov {r4}.z, {r5}.w
break
endif
lt {r6}.y, {r6}.x, l(0x00000000)
mov {r5}.yz, {r4}.yyxy
movc {r4}.xy, {r6}.yyyy, {r5}.xyxx, {r5}.zxzz
lt {r5}.y, l(0x00000000), {r5}.w
div {r5}.z, {r6}.x, {r5}.w
add {r5}.z, -{r5}.z, {r5}.x
add {r6}.x, {r4}.y, {r4}.x
mul {r6}.x, {r6}.x, l(0x3f000000)
movc {r5}.y, {r5}.y, {r5}.z, {r6}.x
lt {r5}.z, {r4}.x, {r5}.y
lt {r6}.y, {r5}.y, {r4}.y
and {r5}.z, {r5}.z, {r6}.y
movc {r5}.y, {r5}.z, {r5}.y, {r6}.x
add {r5}.z, -{r5}.x, {r5}.y
lt {r5}.z, |{r5}.z|, l(0x358637bd)
if_nz {r5}.z
mov {r5}.x, {r5}.y
mov {r4}.z, {r5}.w
break
endif
mov {r5}.x, {r5}.y
iadd {r4}.w, {r4}.w, l(0x00000001)
mov {r4}.z, {r5}.w
endloop
movc {r2}.xyz, {r0}.wwww, -{r2}.xyzx, {r2}.xyzx
sincos {r3}.x, {r4}.x, {r5}.x
mul {r2}.xyz, {r2}.xyzx, {r3}.xxxx
mad {d}.xyz, {r1}.xyzx, {r4}.xxxx, {r2}.xyzx
sincos {r1}.x, null, {r1}.w
max {r1}.y, {r4}.z, l(0x3089705f)
mul {r1}.x, {r1}.y, {r1}.x
div {r1}.x, {r3}.x, {r1}.x
min {r1}.x, |{r1}.x|, l(0x42c80000)
mov {d}.w, l(0x3f800000)
else
mov {d}.xyz, {r0}.xyzx
mov {d}.w, l(0x00000000)
mov {r1}.x, l(0x3f800000)
endif
else
mov {d}.xyz, {r0}.xyzx
mov {d}.w, l(0x00000000)
mov {r1}.x, l(0x3f800000)
endif
else
movc {d}.w, {r0}.w, l(0x00000000), l(0x3f800000)
mov {d}.xyz, {r0}.xyzx
mov {r1}.x, l(0x3f800000)
endif
mov {m}.x, {r1}.x
mov {m}.yzw, l(0x00000000,0x00000000,0x00000000,0x00000000)
