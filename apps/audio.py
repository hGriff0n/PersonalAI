#!/usr/bin/env python3

import asyncio

import speech_recognition as sr
# from win32com import client as com
from tts.sapi import Sapi as Client

from wit import Wit

from common import logger

# Immediate development work
# TODO: Improve intent extraction/dispatch
  # Implement the ability to play songs through spotify
    # This may be difficult, maybe hardcode in some songs to play
# TODO: Add in broader control over the computer's audio systems
  # Controlling spotify will meet this somewhat
  # Change the "I'm listening" signal to a small beep
  # I want a more general control however (may be out of the scope for this app)
# NOTE: After this work is done, shift over to cli app

# Long term dev work
# TODO: Implement resource contention resolution (accounting for audio usage)
#   Look into adding a "wake word" for these situations
# TODO: Implement voice recognition (probably requires AI)

log = logger.create('audio.log')
log.setLevel(logger.logging.INFO)

client = Wit('CM7NKOIYX5BSFGPOPYFAWZDJTZWEVPSR', logger=log)

audio = {
    'mic': sr.Microphone(),
    'voice': Client(),
    'speaker': None
}

# Main event function which handles input and dispatching
async def run():
    rec = sr.Recognizer()

    with audio['mic'] as source:
        rec.adjust_for_ambient_noise(source)
        print("> ...")
        audio_data = rec.listen(source)

    try:
        query = rec.recognize_google(audio_data)
        log.info("HEARD <{}>".format(query))
        dispatch(query, audio['voice'])

    except sr.UnknownValueError:
        asyncio.ensure_future(run())                            # Since we never entered dispatch, we still need to run
        log.error("Couldn't recognize audio")

    except Exception as e:
        asyncio.ensure_future(run())
        log.error(e)



# Pass along the speech data to determine what to do
def dispatch(query, voice):
    msg = client.message(query)
    log.info("MSG <{}>".format(str(msg)))

    action = msg['entities']
    schedule_run = True

    # Perform intent recognition and dispatch
    if 'intent' in action:
        log.info("INTENTS <{}>".format(action['intent']))

        intent = action['intent'][0]        # TODO: Figure out why 'intent' is an array
        schedule_run = 'stop' != intent['value']
        voice.say("You said \"{}\"".format(query))

    elif 'greetings' in action:
        log.info("GREETING")
        voice.say("Hello")

    elif 'thanks' in action:
        log.info("THANKS")
        voice.say("Thanks")

    elif 'bye' in action:
        log.info("GOODBYE")
        voice.say("Bye")
        schedule_run = False

    else:
        log.info("Unknown Action")
        voice.say("I have no idea what you're saying")

    # Schedule another run unless we need to stop
    if schedule_run:
        asyncio.ensure_future(run())



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
