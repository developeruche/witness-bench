> it is important to note that data like this [`- EL: [witness-bench]: debug_executionWitness -> Block: 235 (175.401042ms)`] did not accout for the time it would take for the program requestion the witness to receive the data.
> it only accounts for the time it took for geth to generate the witness. but is cases two we account for the time it takes for the program requestion the witness to receive the data the reciever being the CL (lighthouse). 
> to do an apples to apples compairison we should introduced another field called `REQUESTOR` to account for the time it takes for the program requestion the witness to receive the data.

### Case One:
1. block0_100mb
- REQUESTOR: [witness-bench]: debug_executionWitness: 3860.9 ms
- EL: [witness-bench]: debug_executionWitness -> Block: 235 (175.401042ms)
- CL: [witness-bench]: engine_newPayload -> Block: 235 (20.129667ms)
- Witness Size: 106.91 MB

2. block0_300mb
- REQUESTOR: [witness-bench]: debug_executionWitness: 11031.4 ms
- EL: [witness-bench]: debug_executionWitness -> Block: 300 (467.979417ms)
- CL: [witness-bench]: engine_newPayload -> Block: 300 (61.476084ms)
- Witness Size: 312.34 MB

3. block0_500mb
- REQUESTOR: [witness-bench]: debug_executionWitness: 22613.6 ms
- EL: [witness-bench]: debug_executionWitness -> Block: 316 (592.908959ms)
- CL: [witness-bench]: engine_newPayload -> Block: 316 (98.931417ms)
- Witness Size: 496.19 MB

4. block8_300mb
- REQUESTOR: [witness-bench]: debug_executionWitness: 11286.7 ms
- EL: [witness-bench]: debug_executionWitness -> Block: 343 (435.7775ms)
- CL: [witness-bench]: engine_newPayload -> Block: 343 (56.23325ms)
- Witness Size: 314.70 MB

5. block32_300mb
- REQUESTOR: [witness-bench]: debug_executionWitness: 11404.2 ms
- EL: [witness-bench]: debug_executionWitness -> Block: 216 (468.639667ms)
- CL: [witness-bench]: engine_newPayload -> Block: 216 (55.123125ms)
- Witness Size: 313.44 MB

6. block64_300mb
- REQUESTOR: [witness-bench]: No data here, 64 block depth is not supported by the current geth implementation.
- EL: [witness-bench]: No data here, 64 block depth is not supported by the current geth implementation.
- CL: [witness-bench]: engine_newPayload -> Block: 122 (65.061041ms)
- Witness Size: 



### Case Two:
1. 100mb.ts
[el-1-geth-lighthouse] INFO [02-09|06:53:19.043] [WITNESS_BENCH] Witness Setup Time       block=128 duration="1.708µs"
[el-1-geth-lighthouse] INFO [02-09|06:53:19.191] [WITNESS_BENCH] Block Execution Time     block=128 duration=147.555708ms
[el-1-geth-lighthouse] INFO [02-09|06:53:19.195] [WITNESS_BENCH] Total Execution+Witness Time block=128 duration=152.638833ms
[el-1-geth-lighthouse] INFO [02-09|06:53:19.333] [WITNESS_BENCH] Approx Serialization Time block=128 duration=137.417084ms

[cl-1-lighthouse-geth] Feb 09 06:53:20.155 INFO  [WITNESS_BENCH] CL Round-Trip Latency: 1.117657167s (Block 128)

2. 300mb.ts
[el-1-geth-lighthouse] INFO [02-09|07:07:31.091] [WITNESS_BENCH] Witness Setup Time       block=199 duration="6.416µs"
[el-1-geth-lighthouse] INFO [02-09|07:07:31.383] [WITNESS_BENCH] Block Execution Time     block=199 duration=291.266708ms
[el-1-geth-lighthouse] INFO [02-09|07:07:31.393] [WITNESS_BENCH] Total Execution+Witness Time block=199 duration=302.780625ms
[el-1-geth-lighthouse] INFO [02-09|07:07:31.878] [WITNESS_BENCH] Approx Serialization Time block=199 duration=484.36125ms

[cl-1-lighthouse-geth] Feb 09 07:07:38.450 INFO  [WITNESS_BENCH] CL Round-Trip Latency: 7.372515629s (Block 199)

3. 500mb.ts
[el-1-geth-lighthouse] INFO [02-09|08:06:43.134] [WITNESS_BENCH] Witness Setup Time       block=493 duration="10.208µs"
[el-1-geth-lighthouse] INFO [02-09|08:06:43.535] [WITNESS_BENCH] Block Execution Time     block=493 duration=400.621ms
[el-1-geth-lighthouse] INFO [02-09|08:06:43.558] [WITNESS_BENCH] Total Execution+Witness Time block=493 duration=425.945125ms
[el-1-geth-lighthouse] INFO [02-09|08:06:45.733] [WITNESS_BENCH] Approx Serialization Time block=493 duration=2.175325335s

[cl-1-lighthouse-geth] Feb 09 08:06:50.348 INFO  [WITNESS_BENCH] CL Round-Trip Latency: 7.237190587s (Block 493)

4. Empty block
[el-1-geth-lighthouse] INFO [02-09|08:09:07.037] [WITNESS_BENCH] Witness Setup Time       block=505 duration="11.792µs"
[el-1-geth-lighthouse] INFO [02-09|08:09:07.038] [WITNESS_BENCH] Block Execution Time     block=505 duration="187.125µs"
[el-1-geth-lighthouse] INFO [02-09|08:09:07.040] [WITNESS_BENCH] Total Execution+Witness Time block=505 duration=2.424833ms
[el-1-geth-lighthouse] INFO [02-09|08:09:07.040] [WITNESS_BENCH] Approx Serialization Time block=505 duration="111.417µs"


[cl-1-lighthouse-geth] Feb 09 08:09:07.040 INFO  [WITNESS_BENCH] CL Round-Trip Latency: 3.934042ms (Block 505)
