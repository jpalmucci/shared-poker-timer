var version = "v1.0.0::";

var offlineFundamentals = [
  // add here the files you want to cache
];

self.addEventListener("install", function (event) {
    //console.log('WORKER: install event in progress.');
    /* Using event.waitUntil(p) blocks the installation process on the provided
       promise. If the promise is rejected, the service worker won't be installed.
    */
    event.waitUntil(
      /* The caches built-in is a promise-based API that helps you cache responses,
         as well as finding and deleting them.
      */
      caches
        /* You can open a cache by name, and this method returns a promise. We use
           a versioned cache name here so that we can remove old cache entries in
           one fell swoop later, when phasing out an older service worker.
        */
        .open(version + 'fundamentals')
        .then(function (cache) {
          /* After the cache is opened, we can fill it with the offline fundamentals.
             The method below will add all resources in `offlineFundamentals` to the
             cache, after making requests for them.
          */
          return cache.addAll(offlineFundamentals);
        })
        .then(function () {
          //console.log('WORKER: install completed');
        })
    );
  });

self.addEventListener("fetch", (event) => {
  event.respondWith(
    caches.match(event.request).then((response) => {
      return response || fetch(event.request);
    })
  );
});

self.addEventListener("push", (event) => {
    const data = event.data ? event.data.json() : {};
    console.log(data);
    const title = data.title || "Notification";
    const options = {
      body: data.body || "You have a new message!",
      // icon: '/icon.png',
      // badge: '/badge.png'
    };
    let result = self.registration.showNotification(title, options);
    event.waitUntil(result);
  });
  
  