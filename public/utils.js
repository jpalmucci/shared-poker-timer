
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
