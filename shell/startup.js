(() => {
  const started = Date.now();
  let failed = false;
  const status = document.getElementById('status');
  const hint = document.getElementById('hint');
  const elapsed = document.getElementById('elapsed');
  window.biankStartup = {
    update({phase, detail}) {
      if (failed || typeof detail !== 'string') return;
      failed = phase === 'error';
      document.body.dataset.state = failed ? 'error' : 'loading';
      document.querySelector('main').setAttribute('aria-busy', String(!failed));
      if (status.textContent !== detail) status.textContent = detail;
      const message = failed
        ? 'Biank no pudo completar el inicio. Conserva tus carpetas y comparte este mensaje con soporte.'
        : phase === 'snapshot'
          ? 'Actualizando versión. Asegurando tus datos antes de continuar…'
          : 'Estamos preparando tu espacio. Espera a que Biank termine de iniciar.';
      if (hint.textContent !== message) hint.textContent = message;
    }
  };
  const timer = setInterval(() => {
    if (failed) { clearInterval(timer); return; }
    const seconds = Math.floor((Date.now() - started) / 1000);
    elapsed.textContent = seconds >= 15 ? 'Iniciando servicios locales…' : '';
  }, 1000);
})();
