FROM rust:slim
## if we use the ubuntu image, it is smaller, but for some reason the vapid signature does not work
COPY target/site /app/site
COPY target/release/pokertimer /app/pokertimer
EXPOSE 8443
EXPOSE 3000
WORKDIR /app
ENV LEPTOS_SITE_ROOT="site"
ENTRYPOINT [ "/app/pokertimer" ]
