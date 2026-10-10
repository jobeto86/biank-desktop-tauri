(() => {
  const started = Date.now();
  let failed = false;
  const status = document.getElementById('status');
  const hint = document.getElementById('hint');
  const elapsed = document.getElementById('elapsed');

  const SPRITES = {
    neutral: '0% 0%',
    blink: '50% 0%',
    look: '100% 0%',
    thinking: '0% 100%',
    talking: '50% 100%',
    ready: '100% 100%'
  };

  class BiankAnimator {
    constructor() {
      this.layerA = document.querySelector('.sprite-layer.layer-a');
      this.layerB = document.querySelector('.sprite-layer.layer-b');
      this.stage = document.querySelector('.avatar-stage');
      this.activeLayer = 'a';
      this.current = 'neutral';
      this.timer = null;
      if (this.layerA && this.layerB) {
        this.layerA.style.backgroundPosition = SPRITES.neutral;
        this.startChoreography();
      }
    }

    setSprite(name) {
      if (!SPRITES[name] || this.current === name) return;
      const nextPos = SPRITES[name];
      if (this.activeLayer === 'a') {
        this.layerB.style.backgroundPosition = nextPos;
        this.layerB.style.opacity = '1';
        this.layerA.style.opacity = '0';
        this.activeLayer = 'b';
      } else {
        this.layerA.style.backgroundPosition = nextPos;
        this.layerA.style.opacity = '1';
        this.layerB.style.opacity = '0';
        this.activeLayer = 'a';
      }
      this.current = name;
    }

    startChoreography() {
      const steps = [
        { t: 400, sprite: 'thinking' },
        { t: 1550, sprite: 'blink' },
        { t: 1900, sprite: 'thinking' },
        { t: 2850, sprite: 'look' }
      ];

      steps.forEach(s => {
        setTimeout(() => {
          if (!failed && this.current !== 'ready') this.setSprite(s.sprite);
        }, s.t);
      });

      this.timer = setInterval(() => {
        if (failed || this.current === 'ready') {
          clearInterval(this.timer);
          return;
        }
        const cycle = ['thinking', 'blink', 'thinking', 'look', 'talking'];
        const idx = Math.floor(Date.now() / 1500) % cycle.length;
        this.setSprite(cycle[idx]);
      }, 1500);
    }

    onPhase(phase) {
      if (phase === 'ready') {
        if (this.timer) clearInterval(this.timer);
        this.setSprite('ready');
        if (this.stage) this.stage.classList.add('ready-celebrate');
      } else if (phase === 'snapshot') {
        this.setSprite('thinking');
      } else if (phase === 'error') {
        if (this.timer) clearInterval(this.timer);
        this.setSprite('thinking');
      }
    }
  }

  const animator = new BiankAnimator();

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
          : phase === 'ready'
            ? '¡Todo listo! Abriendo tu espacio…'
            : 'Estamos preparando tu espacio. Espera a que Biank termine de iniciar.';
      if (hint.textContent !== message) hint.textContent = message;
      animator.onPhase(phase);
    }
  };

  const timer = setInterval(() => {
    if (failed) { clearInterval(timer); return; }
    const seconds = Math.floor((Date.now() - started) / 1000);
    elapsed.textContent = seconds >= 15 ? 'Iniciando servicios locales…' : '';
  }, 1000);
})();
