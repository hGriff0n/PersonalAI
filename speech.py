#!/usr/bin/env python3

# NOTE: Requires PyAudio for use of Microphone

import asyncio
import logging
import os

import speech_recognition as sr
import time

# TODO: Need to come up with a way to force events to optionally wait until a specific event finishes
# This is mainly necessary for cases where the AI needs to produce a response (spoken or written)
# However, it'll be better modelled as a dependency graph/resource contention in the general case
# NOTE: I don't want a "complete stop" either. If I'm playing music, then input should take priority
# If I'm responding to a query, then the response should take priority

# TODO: Switch out input from speech to text-based (a lot faster to develop with)

# TODO: Improve logging capabilities
log = logging.getLogger(__name__)
hdlr = logging.FileHandler('speech.log')        # NOTE: This reuses the file if it already exists
formatter = logging.Formatter('%(asctime)s <%(levelname)s> %(message)s')
hdlr.setFormatter(formatter)
log.addHandler(hdlr)
log.setLevel(logging.INFO)

# NOTE: SpeechRecognition currently doesn't natievly support asyncio
# This is a simple workaround to enable running of events while listening
# Taken from: https://github.com/Uberi/speech_recognition/issues/137
async def listen_async(self, source):
    import threading
    result_future = asyncio.Future()

    def threaded_listen():
        with source as s:
            try:
                audio = self.listen(s)
                loop.call_soon_threadsafe(result_future.set_result, audio)
            except Exception as e:
                loop.call_soon_threadsafe(result_future.set_exception, e)

    listener_thread = threading.Thread(target=threaded_listen)
    listener_thread.daemon = True
    listener_thread.start()
    return await result_future

async def run(loop):
    r = sr.Recognizer()
    m = sr.Microphone()
    with m as source:
        r.adjust_for_ambient_noise(source)

    print("You can start talking now\n")

    while True:
        audio_data = await listen_async(r, m)
        query = r.recognize_google(audio_data)
        log.info("Heard \"{}\"".format(query))

        # TODO: Recognize the query to see if there's anything actionable in it
        # NOTE: Use `asyncio.ensure_future` to run my tasks asynchronously (without needing to wait on them)
        # NOTE: Call `asyncio.sleep(0)` to immediately force a context switch (without waiting on the event)

        if query == "exit":
            break

loop = asyncio.get_event_loop()
try:
    loop.run_until_complete(run(loop))
finally:
    # TODO: Need to wait on all running events before closing (https://medium.com/python-pandemonium/asyncio-coroutine-patterns-beyond-await-a6121486656f)
    loop.close()

# API Documentation: https://github.com/Uberi/speech_recognition/blob/master/reference/library-reference.rst
# https://realpython.com/python-speech-recognition/
