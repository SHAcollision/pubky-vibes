(function () {
  if (window.__pubkyMobileEnhancements) {
    return;
  }
  const navigatorUA = (navigator && navigator.userAgent) || '';
  const isAndroid = /android/i.test(navigatorUA);
  const hasTouch =
    'ontouchstart' in window ||
    (navigator && (navigator.maxTouchPoints > 0 || navigator.msMaxTouchPoints > 0));

  if (!isAndroid || !hasTouch) {
    return;
  }

  window.__pubkyMobileEnhancements = true;

  const LONG_PRESS_MS = 550;
  const TOOLTIP_OFFSET_PX = 28;
  const TOOLTIP_VISIBLE_MS = 2600;
  const TOOLTIP_VIEWPORT_PADDING = 12;
  let longPressTimer = null;
  let activeTarget = null;
  let pendingTarget = null;
  let tooltipHideTimer = null;
  let toastTimer = null;

  const tooltip = document.createElement('div');
  tooltip.className = 'touch-tooltip';
  tooltip.setAttribute('role', 'status');

  const toast = document.createElement('div');
  toast.className = 'touch-toast';
  toast.setAttribute('role', 'status');

  function ensureMounted() {
    if (!document.body) {
      return;
    }
    if (!tooltip.isConnected) {
      document.body.appendChild(tooltip);
    }
    if (!toast.isConnected) {
      document.body.appendChild(toast);
    }
    document.body.classList.add('android-touch');
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', ensureMounted, { once: true });
  } else {
    ensureMounted();
  }

  function clearLongPressTimer() {
    if (longPressTimer !== null) {
      clearTimeout(longPressTimer);
      longPressTimer = null;
    }
    pendingTarget = null;
  }

  function clearHideTimer() {
    if (tooltipHideTimer !== null) {
      clearTimeout(tooltipHideTimer);
      tooltipHideTimer = null;
    }
  }

  function hideTooltip() {
    clearHideTimer();
    tooltip.classList.remove('visible');
    tooltip.classList.remove('below');
    tooltip.removeAttribute('style');
    tooltip.textContent = '';
    activeTarget = null;
    pendingTarget = null;
  }

  function showTooltip(target, text, x, y) {
    ensureMounted();
    clearHideTimer();
    tooltip.textContent = text;

    tooltip.classList.remove('visible');
    tooltip.classList.remove('below');
    tooltip.style.visibility = 'hidden';
    tooltip.style.left = `${Math.round(x)}px`;
    tooltip.style.top = `${Math.round(y)}px`;
    tooltip.classList.add('visible');

    const tooltipRect = tooltip.getBoundingClientRect();
    const viewportHeight = window.innerHeight || document.documentElement.clientHeight || 0;
    let showBelow = false;
    let top = y - TOOLTIP_OFFSET_PX - tooltipRect.height;

    if (top < TOOLTIP_VIEWPORT_PADDING) {
      showBelow = true;
      top = y + TOOLTIP_OFFSET_PX;
    } else if (top + tooltipRect.height > viewportHeight - TOOLTIP_VIEWPORT_PADDING) {
      top = Math.max(
        TOOLTIP_VIEWPORT_PADDING,
        viewportHeight - tooltipRect.height - TOOLTIP_VIEWPORT_PADDING
      );
    }

    if (showBelow && top + tooltipRect.height > viewportHeight - TOOLTIP_VIEWPORT_PADDING) {
      showBelow = false;
      top = Math.max(
        TOOLTIP_VIEWPORT_PADDING,
        y - TOOLTIP_OFFSET_PX - tooltipRect.height
      );
    }

    tooltip.classList.toggle('below', showBelow);
    tooltip.style.left = `${Math.round(x)}px`;
    tooltip.style.top = `${Math.round(top)}px`;
    tooltip.style.visibility = '';
    tooltip.classList.add('visible');
    tooltipHideTimer = window.setTimeout(() => {
      hideTooltip();
    }, TOOLTIP_VISIBLE_MS);
    activeTarget = target;
  }

  function scheduleTooltip(event, target, text) {
    clearLongPressTimer();
    const touch = event.touches && event.touches[0];
    if (!touch) {
      return;
    }
    const { clientX: x, clientY: y } = touch;
    pendingTarget = target;
    longPressTimer = window.setTimeout(() => {
      longPressTimer = null;
      pendingTarget = null;
      showTooltip(target, text, x, y);
    }, LONG_PRESS_MS);
  }

  function cancelTooltip(immediate = true) {
    clearLongPressTimer();
    if (immediate) {
      hideTooltip();
    }
  }

  document.addEventListener(
    'touchstart',
    (event) => {
      const target = event.target.closest('[data-touch-tooltip]');
      if (!target) {
        cancelTooltip();
        return;
      }
      const text = target.getAttribute('data-touch-tooltip');
      if (!text) {
        cancelTooltip();
        return;
      }
      if (
        (activeTarget && activeTarget !== target) ||
        (pendingTarget && pendingTarget !== target)
      ) {
        cancelTooltip();
      }
      scheduleTooltip(event, target, text);
    },
    { passive: true }
  );

  document.addEventListener(
    'touchmove',
    (event) => {
      if (!activeTarget && pendingTarget === null) {
        return;
      }
      const current = event.target.closest('[data-touch-tooltip]');
      if (!current) {
        cancelTooltip();
        return;
      }
      if ((activeTarget && current !== activeTarget) || (pendingTarget && current !== pendingTarget)) {
        cancelTooltip();
      }
    },
    { passive: true }
  );

  document.addEventListener(
    'touchend',
    () => {
      cancelTooltip(false);
    },
    { passive: true }
  );
  document.addEventListener(
    'touchcancel',
    () => {
      cancelTooltip();
    },
    { passive: true }
  );
  document.addEventListener('contextmenu', (event) => {
    if (event.target.closest('[data-touch-tooltip]')) {
      event.preventDefault();
    }
  });

  function showToast(message) {
    ensureMounted();
    toast.textContent = message;
    toast.classList.add('visible');
    if (toastTimer !== null) {
      clearTimeout(toastTimer);
    }
    toastTimer = window.setTimeout(() => {
      toast.classList.remove('visible');
    }, 2000);
  }

  async function copyToClipboard(value) {
    if (!value) {
      return false;
    }
    if (navigator.clipboard && navigator.clipboard.writeText) {
      try {
        await navigator.clipboard.writeText(value);
        return true;
      } catch (error) {
        console.warn('Failed to write clipboard', error);
      }
    }
    const helper = document.createElement('textarea');
    helper.value = value;
    helper.setAttribute('readonly', '');
    helper.style.position = 'absolute';
    helper.style.opacity = '0';
    helper.style.pointerEvents = 'none';
    helper.style.left = '-9999px';
    document.body.appendChild(helper);
    helper.select();
    let success = false;
    try {
      success = document.execCommand('copy');
    } catch (error) {
      console.warn('execCommand copy failed', error);
    }
    helper.remove();
    return success;
  }

  document.addEventListener(
    'click',
    (event) => {
      const target = event.target.closest('[data-touch-copy]');
      if (!target) {
        return;
      }
      const value = target.getAttribute('data-touch-copy');
      if (!value) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      copyToClipboard(value).then((ok) => {
        const successMessage = target.getAttribute('data-copy-success') || 'Copied to clipboard';
        const failureMessage = target.getAttribute('data-copy-failure') || 'Unable to copy';
        const feedbackMode = target.getAttribute('data-touch-feedback') || 'toast';
        const message = ok ? successMessage : failureMessage;

        if (!message) {
          return;
        }

        if (feedbackMode === 'tooltip') {
          cancelTooltip();
          const rect = target.getBoundingClientRect();
          const x = rect.left + rect.width / 2;
          const y = rect.top;

          if (Number.isFinite(x) && Number.isFinite(y)) {
            showTooltip(target, message, x, y);
            return;
          }
        }

        showToast(message);
      });
    },
    true
  );
})();
