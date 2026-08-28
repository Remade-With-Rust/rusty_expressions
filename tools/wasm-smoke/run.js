// Run the wasm32 smoke battery under Node.
//
// `cargo check --target wasm32-unknown-unknown` proves the crate compiles.
// This proves it RUNS: the module has no imports, so any trap, missing
// intrinsic or bad relocation shows up here as a thrown error rather than as
// a claim in a README.
const fs = require('fs');
const path = require('path');

const file = path.join(__dirname, 'target/wasm32-unknown-unknown/release/wasm_smoke.wasm');
const bytes = fs.readFileSync(file);

(async () => {
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const imports = WebAssembly.Module.imports(await WebAssembly.compile(bytes));
  if (imports.length) {
    console.log('unexpected host imports:', imports);
    process.exit(1);
  }
  const t0 = process.hrtime.bigint();
  const failures = instance.exports.run();
  const ms = Number(process.hrtime.bigint() - t0) / 1e6;
  console.log(`wasm32 module: ${(bytes.length / 1024).toFixed(0)} KiB, no host imports`);
  console.log(`run() -> ${failures} failing cases in ${ms.toFixed(1)} ms`);
  process.exit(failures === 0 ? 0 : 1);
})().catch((e) => {
  console.error('wasm execution failed:', e);
  process.exit(1);
});
