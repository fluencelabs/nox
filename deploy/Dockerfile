from python:2-alpine as BUILD
RUN apk add gcc musl-dev libffi-dev rust cargo make openssl-dev
copy ./requirements.txt ./requirements.txt
RUN pip install --user -r requirements.txt

from python:2-alpine
copy --from=BUILD /root/.local /root/.local
copy . /deploy
env PATH=/root/.local/bin:$PATH
workdir /deploy
ENTRYPOINT ["fab"]
