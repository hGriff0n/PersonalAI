#!/usr/bin/env python3

import logging
import time

import speech_recognition as sr
# https://github.com/DeepHorizons/tts
import win32com.client as wincl

from common.logger import createLogger, logging

# TODO: Exception safe this running
# TODO: Abstract the handling of query action recognition to insert customization point
# TODO: Add in broader control over the computer's audio systems
# TODO: Implement resource contention resolution (accounting for audio usage)
# TODO: Implement voice recognition (probably requires AI)

log = createLogger('audio.log')
log.setLevel(logging.INFO)

def run():
    rec = sr.Recognizer()
    mic = sr.Microphone()
    voice = wincl.Dispatch("SAPI.SpVoice")

    while True:
        with mic as source:
            rec.adjust_for_ambient_noise(source)
            print("> ...")
            audio_data = rec.listen(source)

        query = rec.recognize_google(audio_data)
        log.info("HEARD <{}>".format(query))

        # TODO: Abstract this action dispatch into a separate control structure
        if query == "exit" or query == "stop":
            log.info("EXITING")
            break
        else:
            voice.Speak("You said \"{}\"".format(query))

# TODO: Make this event process concurrent and distributed
if __name__ == "__main__":
    run()

# API Documentation:
#   SpeechRecognition: https://github.com/Uberi/speech_recognition/blob/master/reference/library-reference.rst
#   SAPI:
