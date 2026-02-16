Here EngineAPI is running in the IPC(Unix domain socket) transport layer not TCP. For this reason only
`Case Two` is important as only `Case One` accounts for the EngineAPI interaction.

### Case Two:
1. 100mb.ts
[el-2-ethrex-lighthouse] 2026-02-16T03:59:52.057837Z  INFO [witness-bench]: Block production time: 16.329042ms for block 278
[el-2-ethrex-lighthouse] 2026-02-16T03:59:52.179166Z  INFO [witness-bench]: Witness generation time: 121.306458ms for block 278
[el-2-ethrex-lighthouse] 2026-02-16T03:59:53.461659Z  INFO [witness-bench]: Block 278 Witness serialization time: 230.570417ms

[cl-2-lighthouse-ethrex] Feb 16 03:59:53.730 INFO  [WITNESS_BENCH] CL Round-Trip Latency: 1.690528875s (Block 278)
[cl-2-lighthouse-ethrex] Feb 16 03:59:53.730 INFO  [WITNESS_BENCH] CL Received Witness Size: 59855805 bytes (Block 278)

2. 300mb.ts
[el-2-ethrex-lighthouse] 2026-02-16T03:57:04.116088Z  INFO [witness-bench]: Block production time: 48.308084ms for block 264
[el-2-ethrex-lighthouse] 2026-02-16T03:57:04.709912Z  INFO [witness-bench]: Witness generation time: 593.801084ms for block 264
[el-2-ethrex-lighthouse] 2026-02-16T03:57:07.582068Z  INFO [witness-bench]: Block 264 Witness serialization time: 823.200167ms

[cl-2-lighthouse-ethrex] Feb 16 03:57:08.526 INFO  [WITNESS_BENCH] CL Round-Trip Latency: 4.46132196s (Block 264)
[cl-2-lighthouse-ethrex] Feb 16 03:57:08.526 INFO  [WITNESS_BENCH] CL Received Witness Size: 188012746 bytes (Block 264)

3. 500mb.ts
[el-2-ethrex-lighthouse] 2026-02-16T03:47:28.171414Z  INFO [witness-bench]: Block production time: 69.6865ms for block 218
[el-2-ethrex-lighthouse] 2026-02-16T03:47:28.889372Z  INFO [witness-bench]: Witness generation time: 717.934458ms for block 218
[el-2-ethrex-lighthouse] 2026-02-16T03:47:32.828537Z  INFO [witness-bench]: Block 218 Witness serialization time: 1.278138918s

[cl-2-lighthouse-ethrex] Feb 16 03:47:34.099 INFO  [WITNESS_BENCH] CL Round-Trip Latency: 6.003329544s (Block 218)
[cl-2-lighthouse-ethrex] Feb 16 03:47:34.099 INFO  [WITNESS_BENCH] CL Received Witness Size: 294752107 bytes (Block 218)






























