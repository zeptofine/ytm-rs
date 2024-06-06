import sys
from copy import deepcopy
from queue import Queue
from threading import Thread
from typing import TypedDict
from urllib import parse as urlparse

import orjson
from flask import Flask, logging, request
from yt_dlp import YoutubeDL

app = Flask(__name__)


opts = {
    "extract_flat": "discard_in_playlist",
    # "final_ext": "ogg",
    "format": "bestaudio/best",
    "fragment_retries": 10,
    "ignoreerrors": "only_download",
    "outtmpl": {"default": ".tmp_%(id)s.%(ext)s"},
    "postprocessors": [
        {
            "key": "FFmpegExtractAudio",
            "nopostoverwrites": False,
            "preferredcodec": "best",
            "preferredquality": "5",
        },
        # {"key": "FFmpegVideoConvertor", "preferedformat": "vorbis"},
        {"key": "FFmpegConcat", "only_multi_video": True, "when": "playlist"},
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

            # Delete unnecessary keys
            if "heatmap" in info:
                info.pop("heatmap")

            if "entries" in info:
                info["entries"] = list(info["entries"])
            with open("tmp_search.json", "wb") as f:
                f.write(orjson.dumps(info))
            return info
    except Exception as e:
        return str(e)


class DownloadDict(TypedDict):
    url: str
    convert_to: str


# AUDIO FORMAT CHOICES:

# POSTPROCESSORS:
FORMAT_POSTPROCESSORS: dict[str, tuple[str, dict[str, str]]] = {
    # Hmm vorbis doesnt work??
    "vorbis": ("ogg", {"key": "FFmpegVideoConvertor", "preferedformat": "vorbis"}),
    "aac": ("aac", {"key": "FFmpegVideoConvertor", "preferedformat": "aac"}),
    "flac": ("flac", {"key": "FFmpegVideoConvertor", "preferedformat": "flac"}),
    "mp3": ("mp3", {"key": "FFmpegVideoConvertor", "preferedformat": "mp3"}),
    "wav": ("wav", {"key": "FFmpegVideoConvertor", "preferedformat": "wav"}),
}


# STREAMING DOWNLOAD PROGRESS:
def __download():
    ...
    # q = Queue()

    # def progress():
    #     idx = 0
    #     while True:
    #         item = q.get()
    #         if item is StopIteration:
    #             break
    #         js = orjson.dumps(item)
    #         with open(f"tmp_{idx}.json", "wb") as f:
    #             f.write(js)
    #             idx += 1

    #         yield orjson.dumps(item)
    #         yield "\n"

    # def add_p2q(progress):
    #     q.put(progress)

    # def request_info(json):
    #     try:
    #         opts_ = opts.copy()
    #         opts_["progress_hooks"] = [add_p2q]
    #             q.put(StopIteration)
    #     except Exception as e:
    #         q.put(repr(e))
    #         q.put(StopIteration)

    # t = Thread(target=request_info, args=(json,))
    # t.daemon = True
    # t.start()
    # return (progress(), {"Content-Type": "application/x-ndjson"})


@app.route("/download", methods=["POST"])
def download():
    json = request.json
    postprocessor: tuple[str, dict[str, str]] | None = None
    if "convert_to" in json:
        if (c2 := json["convert_to"]) in FORMAT_POSTPROCESSORS:
            postprocessor = FORMAT_POSTPROCESSORS[c2]

    try:
        opts_ = deepcopy(opts)
        if postprocessor is not None:
            opts_["postprocessors"].insert(0, postprocessor[1])
            opts_["final_ext"] = postprocessor[0]
        print(opts_)
        with YoutubeDL(opts_) as ytdl:
            info = ytdl.extract_info(
                download=True,
                url=json["url"],
            )
            # Delete unnecessary keys
            if "heatmap" in info:
                info.pop("heatmap")

            with open("tmp_download.json", "wb") as f:
                f.write(orjson.dumps(info))
            return info

    except Exception as e:
        return str(e)


if __name__ == "__main__":
    print(sys.argv)
    port = 55001
    if len(sys.argv) > 1:
        port = int(sys.argv[1])
    app.run(port=port)
