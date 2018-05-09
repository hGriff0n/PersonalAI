#!/usr/bin/env python3

# NOTE: Requires PyAudio for use of Microphone

import asyncio

import speech_recognition as sr
import time

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

async def run():
    r = sr.Recognizer()
    m = sr.Microphone()
    r.adjust_for_ambient_noise(m)

    print("You can start talking now\n")

    while True:
        audio_data = await listen_async(r, m)
        query = r.recognize_google(audio_data)
        print("> {}".format(query))
        if query == "quit":
            break

loop = asyncio.get_event_loop()
try:
    loop.run_until_complete(run())
finally:
    loop.close()

# API Documentation: https://github.com/Uberi/speech_recognition/blob/master/reference/library-reference.rst
# https://realpython.com/python-speech-recognition/
