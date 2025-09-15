
async function requestPushManager() {
  if (!('Notification' in window)) {
    throw new Error('This browser does not support notifications.');
  }

  if (!('serviceWorker' in navigator)) {
    throw new Error('Service workers are not supported in this browser.');
  }

  if (Notification.permission == 'default') {
    await Notification.requestPermission();
  }
  if (Notification.permission !== 'granted') {
    throw new Error('Notification permission not granted.');
  }

  const registration = await navigator.serviceWorker.register("/service-worker.js");

  if (!registration.pushManager)
    throw new Error('PushManager is not available.');
  
  return registration.pushManager;
}

async function startNotifications() {
  let pm = await requestPushManager();
  let cur = await pm.getSubscription();
  if (cur)
    await cur.unsubscribe();
  let sub = await pm.subscribe({ userVisibleOnly: true, applicationServerKey: "BM7EadIlCgfqJABkpI9L0OsbkyZfL1BnEzjBlYpPAoZt-kDpByG3waoERsCLofkeqRsFBRfbgdJ7ccbSb_oxBf8" });
  return sub.toJSON();
};

async function stopNotifications() {
  let pm = await requestPushManager();
  let sub = await pm.getSubscription();
  if (sub) {
    await sub.unsubscribe();
  }
}

async function notificationsSupported() {
  if (!('Notification' in window)) {
    return false;
  }
  if (Notification.permission == 'denied')
    return false;

  if (!('serviceWorker' in navigator)) {
    return false;
  }

  navigator.serviceWorker.register("/service-worker.js");

  const registration = await navigator.serviceWorker.ready;

  if (!registration.pushManager) {
    return false;
  }

  return true;
}

// Universal wake lock with fallback
class UniversalWakeLock {
  constructor() {
    this.method = null;
    this.wakeLock = null;
    this.video = null;
    this.enabled = false;
    this.visibilityHandler = null;
  }

  async enable() {
    this.enabled = true;

    // Try native Wake Lock API first
    if ('wakeLock' in navigator) {
      try {
        this.wakeLock = await navigator.wakeLock.request('screen');
        this.method = 'native';
        console.log('Using native Wake Lock API');

        // Set up visibility change handler if not already set
        if (!this.visibilityHandler) {
          this.visibilityHandler = () => {
            if (document.visibilityState === 'visible' && this.enabled && !this.wakeLock) {
              navigator.wakeLock.request('screen').then(lock => {
                this.wakeLock = lock;
              });
            }
          };
          document.addEventListener('visibilitychange', this.visibilityHandler);
        }

        return true;
      } catch (err) {
        console.log('Native Wake Lock failed, trying fallback');
      }
    }

    // Fallback to video method (for iOS)
    return this.enableVideoFallback();
  }
  
  enableVideoFallback() {
    try {
      const videoData = 'data:video/mp4;base64,AAAAIGZ0eXBpc29tAAACAGlzb21pc28yYXZjMW1wNDEAAAAIZnJlZQAAAu1tZGF0AAACrQYF//+p3EXpvebZSLeWLNgg2SPu73gyNjQgLSBjb3JlIDE0OCByMjYwMSBhMGNkN2QzIC0gSC4yNjQvTVBFRy00IEFWQyBjb2RlYyAtIENvcHlsZWZ0IDIwMDMtMjAxNSAtIGh0dHA6Ly93d3cudmlkZW9sYW4ub3JnL3gyNjQuaHRtbCAtIG9wdGlvbnM6IGNhYmFjPTEgcmVmPTMgZGVibG9jaz0xOjA6MCBhbmFseXNlPTB4MzoweDExMyBtZT1oZXggc3VibWU9NyBwc3k9MSBwc3lfcmQ9MS4wMDowLjAwIG1peGVkX3JlZj0xIG1lX3JhbmdlPTE2IGNocm9tYV9tZT0xIHRyZWxsaXM9MSA4eDhkY3Q9MSBjcW09MCBkZWFkem9uZT0yMSwxMSBmYXN0X3Bza2lwPTEgY2hyb21hX3FwX29mZnNldD0tMiB0aHJlYWRzPTEgbG9va2FoZWFkX3RocmVhZHM9MSBzbGljZWRfdGhyZWFkcz0wIG5yPTAgZGVjaW1hdGU9MSBpbnRlcmxhY2VkPTAgYmx1cmF5X2NvbXBhdD0wIGNvbnN0cmFpbmVkX2ludHJhPTAgYmZyYW1lcz0zIGJfcHlyYW1pZD0yIGJfYWRhcHQ9MSBiX2JpYXM9MCBkaXJlY3Q9MSB3ZWlnaHRiPTEgb3Blbl9nb3A9MCB3ZWlnaHRwPTIga2V5aW50PTI1MCBrZXlpbnRfbWluPTEwIHNjZW5lY3V0PTQwIGludHJhX3JlZnJlc2g9MCByY19sb29rYWhlYWQ9NDAgcmM9Y3JmIG1idHJlZT0xIGNyZj0yMy4wIHFjb21wPTAuNjAgcXBtaW49MCBxcG1heD02OSBxcHN0ZXA9NCBpcF9yYXRpbz0xLjQwIGFxPTE6MS4wMACAAAAAD2WIhAA3//727P4PyvgAAAMAAAMAAAMAAAMAAAMAAAMAAAMAPBiJ';
      
      this.video = document.createElement('video');
      this.video.setAttribute('playsinline', '');
      this.video.src = videoData;
      this.video.loop = true;
      this.video.muted = true;
      this.video.style.cssText = 'position:fixed;left:-100px;top:-100px;width:1px;height:1px;';
      
      document.body.appendChild(this.video);
      
      return this.video.play().then(() => {
        this.method = 'video';
        console.log('Using video fallback for wake lock');
        return true;
      }).catch(err => {
        console.error('Video fallback failed:', err);
        this.disable();
        return false;
      });
    } catch (err) {
      console.error('Wake lock fallback error:', err);
      return false;
    }
  }
  
  async disable() {
    this.enabled = false;

    if (this.wakeLock) {
      await this.wakeLock.release();
      this.wakeLock = null;
    }
    if (this.video) {
      this.video.pause();
      this.video.remove();
      this.video = null;
    }
    this.method = null;
  }
  
  isEnabled() {
    return this.enabled && this.method !== null;
  }
}

// Global wake lock instance
const globalWakeLock = new UniversalWakeLock();

// Functions to be called from WASM
async function enableWakeLock() {
  return await globalWakeLock.enable();
}

async function disableWakeLock() {
  await globalWakeLock.disable();
  return true;
}

function isWakeLockEnabled() {
  return globalWakeLock.isEnabled();
}
