import './style.css';
import init, { verify_inclusion } from '../../wasm-core/pkg/reality_wasm_core.js';

async function bootstrap() {
  await init(new URL('../../wasm-core/pkg/reality_wasm_core_bg.wasm', import.meta.url));

  const root = document.querySelector<HTMLDivElement>('#app');
  if (!root) {
    throw new Error('missing #app container');
  }

  root.innerHTML = `
    <main class="container">
      <h1>RealityLog Proof Verifier</h1>
      <p>Paste a JSON proof payload that matches <code>VerifyRequest</code> and click verify.</p>
      <textarea placeholder="{\n  \"index\": 0,\n  ...\n}" spellcheck="false"></textarea>
      <div class="actions">
        <button type="button">Verify</button>
        <button type="button" data-action="sample">Load Sample</button>
      </div>
      <pre class="output">Awaiting proof…</pre>
    </main>
  `;

  const textarea = root.querySelector<HTMLTextAreaElement>('textarea');
  const verifyButton = root.querySelector<HTMLButtonElement>('button');
  const sampleButton = root.querySelector<HTMLButtonElement>('button[data-action="sample"]');
  const output = root.querySelector<HTMLPreElement>('pre.output');

  if (!textarea || !verifyButton || !sampleButton || !output) {
    throw new Error('failed to initialise verifier UI');
  }

  verifyButton.addEventListener('click', () => {
    const proof = textarea.value.trim();
    if (!proof) {
      output.textContent = 'Provide a proof JSON payload to verify.';
      output.dataset.state = 'warn';
      return;
    }

    try {
      const isValid = verify_inclusion(proof);
      if (isValid) {
        output.textContent = 'Proof valid ✅';
        output.dataset.state = 'ok';
      } else {
        output.textContent = 'Proof invalid ❌';
        output.dataset.state = 'err';
      }
    } catch (err) {
      output.textContent = `Verification error: ${(err as Error).message}`;
      output.dataset.state = 'err';
    }
  });

  sampleButton.addEventListener('click', () => {
    textarea.value = JSON.stringify(
      {
        index: 0,
        leaf: 'f0d79f7fbb79db1def420069cb3547413ba840b048f185a26933d9e463c8f59a',
        root: 'f0d79f7fbb79db1def420069cb3547413ba840b048f185a26933d9e463c8f59a',
        path: [],
      },
      null,
      2,
    );
    output.textContent = 'Sample proof loaded. Click verify to evaluate.';
    output.dataset.state = 'info';
  });
}

bootstrap().catch((err) => {
  console.error('failed to bootstrap verifier', err);
  const root = document.querySelector<HTMLDivElement>('#app');
  if (root) {
    root.innerHTML = '<pre class="output" data-state="err">Failed to load WASM verifier.</pre>';
  }
});
