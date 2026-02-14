

### Case One:
1. block0_100mb
- REQUESTOR: [witness-bench]: debug_executionWitness: 1160.6 ms
- EL: [witness-bench]: debug_executionWitness -> Block: 165 (115.688167ms)
- CL: [witness-bench]: engine_newPayload -> Block: 165 (15.527542ms)
- Witness Size: 107.01 MB

2. block0_300mb
- REQUESTOR: [witness-bench]: debug_executionWitness: 3176.7 ms
- EL: [witness-bench]: debug_executionWitness -> Block: 192 (375.633584ms)
- CL: [witness-bench]: engine_newPayload -> Block: 192 (60.011583ms)
- Witness Size: 312.65 MB

3. block0_500mb
- REQUESTOR: [witness-bench]: debug_executionWitness: 4848.1 ms
- EL: [witness-bench]: debug_executionWitness -> Block: 206 (449.540959ms)
- CL: [witness-bench]: engine_newPayload -> Block: 206 (52.374666ms)
- Witness Size: 496.69 MB

4. block8_300mb
- REQUESTOR: [witness-bench]: debug_executionWitness: 2881.9 ms
- EL: [witness-bench]: debug_executionWitness -> Block: 223 (377.344001ms)
- CL: [witness-bench]: engine_newPayload -> Block: 223 (35.578125ms)
- Witness Size: 312.65 MB

5. block32_300mb
- REQUESTOR: [witness-bench]: debug_executionWitness: 2745.4 ms
- EL: [witness-bench]: debug_executionWitness -> Block: 260 (307.748209ms)
- CL: [witness-bench]: engine_newPayload -> Block: 260 (32.384042ms)
- Witness Size: 312.65 MB

6. block64_300mb
- REQUESTOR: [witness-bench]: debug_executionWitness: No data here, 64 block depth is not supported by the current
- EL: [witness-bench]: No data here, 64 block depth is not supported by the current ethrex implementation. (state root missing).
- CL: [witness-bench]: engine_newPayload -> Block: 371 (38.014959ms)
- Witness Size: no data




### Case Two:
1. 100mb.ts
[el-2-ethrex-lighthouse] 2026-02-14T18:29:14.053240Z  INFO [witness-bench]: Block production time: 13.465042ms for block 163
[el-2-ethrex-lighthouse] 2026-02-14T18:29:14.142674Z  INFO [witness-bench]: Witness generation time: 89.41125ms for block 163
[el-2-ethrex-lighthouse] 2026-02-14T18:29:14.053240Z  INFO [witness-bench]: Block 163 Witness serialization time: 374.20725ms


[cl-2-lighthouse-ethrex] Feb 14 18:29:15.155 INFO  [WITNESS_BENCH] CL Round-Trip Latency: 1.117136751s (Block 163)
[cl-2-lighthouse-ethrex] Feb 14 18:29:15.156 INFO  [WITNESS_BENCH] CL Received Witness Size: 65776391 bytes (Block 163)

2. 300mb.ts
[el-2-ethrex-lighthouse] 2026-02-14T18:35:02.104486Z  INFO [witness-bench]: Block production time: 33.690542ms for block 192
[el-2-ethrex-lighthouse] 2026-02-14T18:35:02.381765Z  INFO [witness-bench]: Witness generation time: 277.26075ms for block 192
[el-2-ethrex-lighthouse] 2026-02-14T18:35:07.271033Z  INFO [witness-bench]: Block 192 Witness serialization time: 1.308214334s

[cl-2-lighthouse-ethrex] Feb 14 18:35:08.862 INFO  [WITNESS_BENCH] CL Round-Trip Latency: 6.794218669s (Block 192)
[cl-2-lighthouse-ethrex] Feb 14 18:35:08.862 INFO  [WITNESS_BENCH] CL Received Witness Size: 197328345 bytes (Block 192)

3. 500mb.ts
[el-2-ethrex-lighthouse] 2026-02-14T18:38:38.188493Z  INFO [witness-bench]: Block production time: 60.488167ms for block 210
[el-2-ethrex-lighthouse] 2026-02-14T18:38:38.716553Z  INFO [witness-bench]: Witness generation time: 528.015708ms for block 210
[el-2-ethrex-lighthouse] 2026-02-14T18:38:45.022197Z  INFO [witness-bench]: Block 210 Witness serialization time: 1.499040542s

[cl-2-lighthouse-ethrex] Feb 14 18:38:46.254 INFO  [WITNESS_BENCH] CL Round-Trip Latency: 8.130902461s (Block 210)
[cl-2-lighthouse-ethrex] Feb 14 18:38:46.254 INFO  [WITNESS_BENCH] CL Received Witness Size: 338319285 bytes (Block 210)


> It is important to note what happen in the witness serialization time buffer is first the `ExecutionWitness` is encoded into RPL, this RLP string is them annotated as a `hex` string which is then sent to the CL in a JSON format.