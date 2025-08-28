FROM rust:1
COPY target/site /app/site
COPY target/release/pokertimer /app/pokertimer
EXPOSE 8443
EXPOSE 3000
WORKDIR /app
ENV LEPTOS_SITE_ROOT="site"
ENTRYPOINT [ "/app/pokertimer" ]
