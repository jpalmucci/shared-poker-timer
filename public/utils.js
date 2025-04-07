
async function requestPushManager() {
    if (!('Notification' in window)) {
      throw new Error('This browser does not support notifications.');
    }
  
    if (!('serviceWorker' in navigator)) {
      throw new Error('Service workers are not supported in this browser.');
    }

    const permission = await Notification.requestPermission();
    if (permission !== 'granted') {
      throw new Error('Notification permission not granted.');
    }
  
    navigator.serviceWorker.register("/service-worker.js");
  
    const registration = await navigator.serviceWorker.ready;
  
    if (!registration.pushManager) {
      throw new Error('PushManager is not available.');
    }
  
    return registration.pushManager;
  }
