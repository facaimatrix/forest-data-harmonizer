// Minimal test app - just get the UI rendering
console.log('Minimal app starting...');

// Simple state
const state = {
  step: 0,
  filePath: null,
};

// Minimal render
function render() {
  console.log('render() called, step:', state.step);
  
  // Render step indicator
  const indicator = document.getElementById('step-indicator');
  if (indicator) {
    indicator.innerHTML = '<div class="step-pip active"><div class="num">0</div>Load</div>';
  } else {
    console.warn('step-indicator not found');
  }

  // Render main content
  const main = document.getElementById('main');
  if (main) {
    main.innerHTML = `
      <div class="step-content">
        <h2>Load dataset file</h2>
        <p class="step-desc">Choose a CSV, XLSX, or Parquet file.</p>
        <button class="btn btn-primary btn-lg" id="pick-file">Choose file…</button>
      </div>
    `;
  } else {
    console.warn('main element not found');
  }

  // Render nav
  const nav = document.getElementById('nav');
  if (nav) {
    nav.innerHTML = `
      <button class="btn btn-ghost">← Back</button>
      <span class="nav-hint">Ready to load file</span>
      <button class="btn btn-primary">Next →</button>
    `;
  } else {
    console.warn('nav element not found');
  }

  // Attach handlers
  const fileBtn = document.getElementById('pick-file');
  if (fileBtn) {
    fileBtn.addEventListener('click', () => {
      alert('File picker clicked (not implemented in minimal version)');
    });
  }
}

// Initialize
console.log('About to call render()');
try {
  render();
  console.log('✓ render() completed');
} catch (e) {
  console.error('✗ render() failed:', e);
  const main = document.getElementById('main');
  if (main) {
    main.innerHTML = '<div style="color: red; padding: 20px; font-family: monospace;">' +
      'Error: ' + (e.message || String(e)) + '</div>';
  }
}
