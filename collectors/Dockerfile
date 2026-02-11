FROM python:3.12-slim

RUN pip install --no-cache-dir curl_cffi click

COPY . /app/
WORKDIR /app

ENTRYPOINT ["python", "hiringcafe.py"]
