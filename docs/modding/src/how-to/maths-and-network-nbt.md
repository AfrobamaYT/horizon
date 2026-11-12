# Maths and network NBT

`mod_api::strict_math` provides platform-independent StrictMath operations:
`sin`, `cos`, `tan`, `exp`, `pow`, `log`, `log10`, `cbrt`, `hypot`, `atan`,
`asin`, `acos` and `atan2`. These are pure functions with identical result bits
on native and wasm (NaN payloads are unspecified). The five `sin`/`cos`/`tan`/
`exp`/`pow` additions use fdlibm's ordered IEEE double algorithms, including
full large-argument trigonometric reduction, signed zeros, subnormals and
overflow/underflow. They use no FMA or platform/libm calls and allocate no heap
memory. Guest code links them locally: there is no Host call per operation.
The AArch64 FMA variants are deliberately not part of this StrictMath set.

The committed literal reference table and deterministic seeded generator
(`tools/generate_strict_math_vectors.py`) use only public JDK `StrictMath`
calls, not engine outputs. Native tests and the real compiled-wasm fixture
compare exact bits, except that NaNs are compared by classification. The table
was generated with OpenJDK `27.0.1.0.3` and contains 768 rows for each unary
function and 1,309 power rows. Local architecture evidence
must be reported separately: cross-platform claims require running these
same vectors on x86_64, AArch64 and wasm32, not just compiling the other target.

`mod_api::java_math` exposes every engine `jmath` free function for native mods,
including `round` and `round_f32`. These are the paths used for vanilla `Math`
call sites; rounding can differ by machine, and universal HotSpot bit equality
is not promised. Calling them on wasm fails to compile and points to `strict_math`.

`Tag::to_network_bytes()` writes the engine's nameless network root, preserving
NaN bits and modified UTF-8 (including unpaired UTF-16 surrogates).
Reading follows the engine codec: negative floating zero normalizes to positive
zero, and mixed-list compound wrappers are unwrapped.
`Tag::from_network_bytes(&bytes, None)` returns `(tag, consumed_bytes)`;
bytes following the root remain unread. `Tag::End` encodes as `[0]` and represents
vanilla's null marker. Errors have the exported `mod_api::NbtError` type.

The default read quota is `mod_api::NETWORK_NBT_QUOTA` (2,097,152 bytes), matching
vanilla 26.3 `ByteBufCodecs.TAG` and `NbtAccounter.defaultQuota`. It counts heap
charges, not wire bytes: primitive tag overhead, array contents, UTF-16 strings
and compound keys, list references, and distinct compound entries. Pass
`Some(larger_quota)` for trusted data. Both methods allow 512 container levels;
the writer refuses strings over 65,535 modified UTF-8 bytes. Vanilla applies
heap quotas on reads, so writing does not impose a heap quota.

The implementation follows vanilla 26.3 `NbtIo.readAnyTag`/`writeAnyTag`,
`FriendlyByteBuf.readNbt`/`writeNbt`, `NbtAccounter`, and each `TagType.load`.
It shares the engine reader and writer. These explicit codec calls cost time
proportional to the encoded data and add no tick-time work or Host calls.
