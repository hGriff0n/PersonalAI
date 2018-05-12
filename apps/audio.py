#!/usr/bin/env python3

import asyncio

import speech_recognition as sr
# https://github.com/DeepHorizons/tts
import win32com.client as wincl

from common import logger

# TODO: Exception safe this running loop
# TODO: Add in broader control over the computer's audio systems
# TODO: Implement resource contention resolution (accounting for audio usage)
# TODO: Implement voice recognition (probably requires AI)

log = logger.create('audio.log')
log.setLevel(logger.logging.INFO)

voice = wincl.Dispatch("SAPI.SpVoice")


# Main event function which handles input and dispatching
async def run():
    rec = sr.Recognizer()

    with sr.Microphone() as source:
        rec.adjust_for_ambient_noise(source)
        print("> ...")
        audio_data = rec.listen(source)

    query = rec.recognize_google(audio_data)
    log.info("HEARD <{}>".format(query))

    dispatch(query, voice)


# Pass along the speech data to determine what to do
def dispatch(query, voice):
    if not (query == "exit" or query == "stop"):
        asyncio.ensure_future(run())

    voice.Speak("You said \"{}\"".format(query))


# TODO: Make this event process concurrent and distributed
if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(run())          # Runs until there are no more functions left to call

# API Documentation:
#   SpeechRecognition: https://github.com/Uberi/speech_recognition/blob/master/reference/library-reference.rst
#   SAPI:
