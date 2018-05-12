#!/usr/bin/env python3

# NOTE: Requires PyAudio for use of Microphone

import asyncio
import logging
import os

import speech_recognition as sr
import time

# https://github.com/DeepHorizons/tts
import win32com.client as wincl

# TODO: Add in broader control over the computer's audio systems
# TODO: Implement resource contention resolution (accounting for audio usage)
# TODO: Implement voice recognition (probably requires AI)


# TODO: Improve logging capabilities
def createLogger(file):
    hdlr = logging.FileHandler(file)
    fmt = logging.Formatter('%(asctime)s <%(levelname)s> %(message)s')
    hdlr.setFormatter(fmt)

    logger = logging.getLogger(__name__)
    logger.addHandler(hdlr)
    return logger

log = createLogger('speech.log')
log.setLevel(logging.INFO)

# NOTE: I could make all this code single threaded and I wouldn't lose much functionality
# Any gain would come fairly late in the process anyways (when the systems more fleshed out)


# NOTE: SpeechRecognition currently doesn't natievly support asyncio
# This is a simple workaround to enable running of events while listening
# Taken from: https://github.com/Uberi/speech_recognition/issues/137
async def listen_async(self, source):
    import threading
    result_future = asyncio.Future()

    def threaded_listen():
        with source as s:
            # with (yield from audio_lock):
            try:
                audio = self.listen(s)
                loop.call_soon_threadsafe(result_future.set_result, audio)
            except Exception as e:
                loop.call_soon_threadsafe(result_future.set_exception, e)

    listener_thread = threading.Thread(target=threaded_listen)
    listener_thread.daemon = True
    listener_thread.start()
    return await result_future

async def recognize(query, ai_voice):
    log.info("recognizing")
    ai_voice.Speak("You said \"{}\"".format(query))

async def run(loop):
    r = sr.Recognizer()
    m = sr.Microphone()
    with m as source:
        r.adjust_for_ambient_noise(source)

    print("You can start talking now\n")
    ai_voice = wincl.Dispatch("SAPI.SpVoice")

    while True:
        audio_data = await listen_async(r, m)
        query = r.recognize_google(audio_data)
        log.info("Heard \"{}\"".format(query))

        # TODO: Recognize the query to see if there's anything actionable in it
        # NOTE: Use `asyncio.ensure_future` to run my tasks asynchronously (without needing to wait on them)
        # NOTE: Call `asyncio.sleep(0)` to immediately force a context switch (without waiting on the event)

        if query == "exit" or query == "stop":
            break

        asyncio.ensure_future(recognize(query, ai_voice), loop=loop)
        asyncio.sleep(0)



# TODO: Adapt this for the more limited role this app will handle
loop = asyncio.get_event_loop()
try:
    loop.run_until_complete(run(loop))
finally:
    # TODO: Need to wait on all running events before closing (https://medium.com/python-pandemonium/asyncio-coroutine-patterns-beyond-await-a6121486656f)
    loop.close()


# API Documentation:
#   SpeechRecognition: https://github.com/Uberi/speech_recognition/blob/master/reference/library-reference.rst
#   Asyncio:
#   SAPI:
