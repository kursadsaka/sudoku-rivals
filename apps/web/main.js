import init, { validate_board } from './pkg/sudoku_wasm.js';
const output = document.querySelector('#result');
try {
  await init();
  const button = document.querySelector('#validate');
  button.disabled = false;
  output.textContent = 'Tahta denetlenmeye hazır.';
  button.addEventListener('click', () => {
    const values = document.querySelector('#board').value.trim().split(/\s+/);
    if (values.length !== 36 || values.some(value => !/^[0-6]$/.test(value))) {
      output.textContent = '0 ile 6 arasında 36 sayı girin.';
      return;
    }
    output.textContent = validate_board(Uint8Array.from(values, Number), 6) ? 'Çakışma yok.' : 'Tahtada çakışma var.';
  });
} catch (error) { output.textContent = `Çekirdek yüklenemedi: ${error.message}`; }
