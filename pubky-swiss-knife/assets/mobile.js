(function () {
  if (window.__pubkyMobileEnhancements) {
    return;
  }
  window.__pubkyMobileEnhancements = true;

  const LONG_PRESS_MS = 550;
  let longPressTimer = null;
  let activeTarget = null;
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

  function clearTooltipTimers() {
    if (longPressTimer !== null) {
      clearTimeout(longPressTimer);
      longPressTimer = null;
    }
    if (tooltipHideTimer !== null) {
      clearTimeout(tooltipHideTimer);
      tooltipHideTimer = null;
    }
  }

  function hideTooltip() {
    tooltip.classList.remove('visible');
    tooltip.removeAttribute('style');
    tooltip.textContent = '';
    activeTarget = null;
  }

  function showTooltip(target, text, x, y) {
    ensureMounted();
    tooltip.textContent = text;
    tooltip.style.left = `${Math.round(x)}px`;
    tooltip.style.top = `${Math.round(y - 12)}px`;
    tooltip.classList.add('visible');
    tooltipHideTimer = window.setTimeout(() => {
      tooltip.classList.remove('visible');
    }, 2000);
    activeTarget = target;
  }

  function scheduleTooltip(event, target, text) {
    clearTooltipTimers();
    const touch = event.touches && event.touches[0];
    if (!touch) {
      return;
    }
    longPressTimer = window.setTimeout(() => {
      showTooltip(target, text, touch.clientX, touch.clientY);
    }, LONG_PRESS_MS);
  }

  function cancelTooltip() {
    clearTooltipTimers();
    hideTooltip();
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
        return;
      }
      scheduleTooltip(event, target, text);
    },
    { passive: true }
  );

  document.addEventListener(
    'touchmove',
    (event) => {
      if (!activeTarget && longPressTimer === null) {
        return;
      }
      const current = event.target.closest('[data-touch-tooltip]');
      if (!current || current !== activeTarget) {
        cancelTooltip();
      }
    },
    { passive: true }
  );

  document.addEventListener('touchend', cancelTooltip, { passive: true });
  document.addEventListener('touchcancel', cancelTooltip, { passive: true });

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
        showToast(ok ? successMessage : failureMessage);
      });
    },
    true
  );
})();
