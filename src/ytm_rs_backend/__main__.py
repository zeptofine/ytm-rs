from typing import TypedDict
from flask import Flask, request
from yt_dlp import YoutubeDL
from queue import Queue
from threading import Thread
import orjson

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
    return "Hello World!"


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
            with open("tmp.txt", "wb") as f:
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
    app.run(port=55001)
