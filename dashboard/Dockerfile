# Docker build should be launched from parent directory this way:
# docker build -f dashboard/Dockerfile .
#
# Build
FROM node:8.16.0 as build
RUN mkdir -p /usr/src/app
COPY bootstrap /usr/src/app/bootstrap
WORKDIR /usr/src/app/bootstrap
RUN npm install --silent
ENV PATH /usr/src/app/dashboard/node_modules/.bin:$PATH
COPY dashboard /usr/src/app/dashboard
WORKDIR /usr/src/app/dashboard
RUN npm install --silent --ignore-scripts
# Run postinstall manually: https://github.com/npm/npm/issues/17346
RUN npm run typedef:compile
RUN npm run typedef:typechain
RUN npm run typedef:copy
RUN npm run build

# Prod webserver
FROM nginx:stable-alpine
COPY --from=build /usr/src/app/dashboard/build /usr/share/nginx/html
RUN echo $'server { \n\
        listen 3000; \n\
        root /usr/share/nginx/html; \n\
        location / { \n\
            try_files $uri /index.html; \n\
        } \n\
    }' | tee /etc/nginx/conf.d/default.conf
EXPOSE 3000
CMD ["nginx", "-g", "daemon off;"]
