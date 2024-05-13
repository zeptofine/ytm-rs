from queue import Queue
import sys
from threading import Thread
from typing import TypedDict
from urllib import parse as urlparse

import orjson
from flask import Flask, request, logging
from yt_dlp import YoutubeDL


app = Flask(__name__)


opts = {
    "check_formats": "selected",
    "extract_flat": "discard_in_playlist",
    "format": "bestaudio/best",
    "fragment_retries": 10,
    "ignoreerrors": "only_download",
    "outtmpl": {"default": "cache/%(id)s"},
    "postprocessors": [
        {
            "key": "FFmpegExtractAudio",
            "preferredquality": "5",
        },
        {
            "key": "FFmpegConcat",
            "only_multi_video": True,
            "when": "playlist",
        },
    ],
    "retries": 10,
}


@app.route("/")
def main():
    return "YTM_RS_BACKEND"


class RequestInfoDict(TypedDict):
    url: str
    process: bool


@app.route("/request_info", methods=["POST"])
def request_info():
    json = request.json
    assert json is not None
    try:
        with YoutubeDL(opts) as ytdl:
            info = ytdl.extract_info(
                download=False,
                url=json["url"],
                process=json["process"],
            )
            assert info is not None
            if "entries" in info:
                info["entries"] = list(info["entries"])
            with open("tmp.json", "wb") as f:
                f.write(orjson.dumps(info))
            return info

    except Exception as e:
        return str(e)


def add_query(url: str, params: dict[str, str]):
    url_parts = list(urlparse.urlparse(url))
    query = dict(urlparse.parse_qsl(url_parts[4]))
    query.update(params)
    url_parts[4] = urlparse.urlencode(query)

    return urlparse.urlunparse(url_parts)


@app.route("/search")
def search():
    base_url = "https://music.youtube.com/search"
    assert request.query_string
    parsed_query = dict(urlparse.parse_qsl(request.query_string.decode("utf-8")))
    print(parsed_query)
    url = add_query(base_url, parsed_query)
    print(url)
    try:
        with YoutubeDL(opts) as ytdl:
            info = ytdl.extract_info(url, download=False, process=False)
            assert info is not None
            if "entries" in info:
                info["entries"] = list(info["entries"])
            with open("tmp_search.json", "wb") as f:
                f.write(orjson.dumps(info))
            return info
    except Exception as e:
        return str(e)


class DownloadDict(TypedDict):
    url: str


@app.route("/download", methods=["POST"])
def download():
    json = request.json
    try:
        q = Queue()

        def progress():
            while True:
                item = q.get()
                if item is StopIteration:
                    break
                yield orjson.dumps(item)
                yield "\n"

        def add_p2q(progress):
            q.put(progress)

        def request_info(json):
            try:
                opts_ = opts.copy()
                opts_["progress_hooks"] = [add_p2q]
                with YoutubeDL(opts) as ytdl:
                    response = ytdl.extract_info(
                        download=True,
                        url=json["url"],
                    )
                    q.put(response)
                    q.put(StopIteration)
            except Exception as e:
                q.put(repr(e))
                q.put(StopIteration)

        t = Thread(target=request_info, args=(json,))
        t.daemon = True
        t.start()
        return (progress(), {"Content-Type": "application/x-ndjson"})

    except Exception as e:
        return str(e)


if __name__ == "__main__":
    print(sys.argv)
    port = 55001
    if len(sys.argv) > 1:
        port = int(sys.argv[1])
    app.run(port=port)
