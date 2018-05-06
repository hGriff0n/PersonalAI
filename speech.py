#!/usr/bin/env python3

# NOTE: Requires PyAudio for use of Microphone

import speech_recognition as sr
import time

def callback(recognizer, audio):
    try:
        print("Google thinks you said \"" + recognizer.recognize_google(audio) + "\"")
    except sr.UnknownValueError:
        print("Google could not understand the audio")
    except sr.RequestError as e:
        print("Could not request results from Google: {0}".format(e))

r = sr.Recognizer()
m = sr.Microphone()
with m as source:
    r.adjust_for_ambient_noise(source)

# 'stop_listening' is a function that stops background listening when called
stop_listening = r.listen_in_background(m, callback)
print("You can start talking now")

for _ in range(50):
    time.sleep(0.1)

stop_listening()

# API Documentation: https://github.com/Uberi/speech_recognition/blob/master/reference/library-reference.rst
# https://realpython.com/python-speech-recognition/
