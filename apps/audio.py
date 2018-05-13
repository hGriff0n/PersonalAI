#!/usr/bin/env python3

import asyncio

import speech_recognition as sr
from tts.sapi import Sapi as TtsClient

from wit import Wit

from common import logger

# TODO: Implement Intent Extraction using wit.ai
# TODO: Add in broader control over the computer's audio systems
# TODO: Implement resource contention resolution (accounting for audio usage)
#   May want to add in a "wake word" for these situations
# TODO: Implement voice recognition (probably requires AI)

log = logger.create('audio.log')
log.setLevel(logger.logging.INFO)

voice = TtsClient()
client = Wit('CM7NKOIYX5BSFGPOPYFAWZDJTZWEVPSR', logger=log)

# Main event function which handles input and dispatching
async def run():
    rec = sr.Recognizer()

    with sr.Microphone() as source:
        rec.adjust_for_ambient_noise(source)
        print("> ...")
        audio_data = rec.listen(source)

    try:
        query = rec.recognize_google(audio_data)
        log.info("HEARD <{}>".format(query))
        dispatch(query, voice)

    except sr.UnknownValueError:
        asyncio.ensure_future(run())                            # Since we never entered dispatch, we still need to run
        log.error("Couldn't recognize audio")

    except Exception as e:
        asyncio.ensure_future(run())
        log.error(e)


# Pass along the speech data to determine what to do
def dispatch(query, voice):
    msg = client.message(query)
    action = msg['entities']
    schedule_run = True

    # Perform intent recognition and dispatch
    if 'intent' in action:
        intent = action['intent'][0]        # TODO: Figure out why 'intent' is an array
        schedule_run = 'stop' != intent['value']

    elif 'greeting' in action:
        log.info("GREETING")
        voice.say("Hello Grayson")

    # Schedule another run unless we need to stop
    if schedule_run:
        asyncio.ensure_future(run())

    log.info("MSG <{}>".format(str(msg)))
    voice.say("You said \"{}\"".format(query))


# TODO: Make this event process concurrent and distributed
if __name__ == "__main__":
    asyncio.ensure_future(run())

    # Run until no more functions are scheduled
    while True:
        log.info("Gathering tasks")
        pending_tasks = [task for task in asyncio.Task.all_tasks() if not task.done()]
        if len(pending_tasks) == 0: break
        asyncio.get_event_loop().run_until_complete(asyncio.gather(*pending_tasks))

# API Documentation:
#   SpeechRecognition: https://github.com/Uberi/speech_recognition/blob/master/reference/library-reference.rst
#   tts.SAPI: https://github.com/DeepHorizons/tts
#   Wit: https://wit.ai/docs
