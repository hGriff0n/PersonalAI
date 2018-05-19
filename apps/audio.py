#!/usr/bin/env python3

import asyncio

# AI recognition
from wit import Wit

# Speech Recognition / TTS
import speech_recognition as sr
# from tts.sapi import Sapi as Client     # Why is this taking a long time
import win32com.client

# Music/general audio
import pyaudio
from pydub import AudioSegment
from pydub.utils import make_chunks

from common import logger
import os

# Long term dev work
# TODO: Improve interactivity of AI
#   For instance, say what "song" is playing when I want to play music
#   Work on modifying the beep tone to be more "pleasant" (its too loud for one)
# TODO: Look at replacing asyncio with Trio
# TODO: Implement a database (or something) to track all local music files
#   This would end up being subsumed by the "backing storage" server though (it's the responsibility)
#   I'll probably have to implement a queuing/thread system to handle networked requests
# TODO: Implement resource contention resolution (accounting for audio usage)
#   Look into adding a "wake word" for these situations
# TODO: Add in spotify playback (once the web api allows it)
#   Look at alternate approaches to music streaming
# TODO: Implement voice recognition (probably requires AI)

log = logger.create('audio.log')
log.setLevel(logger.logging.INFO)

client = Wit('CM7NKOIYX5BSFGPOPYFAWZDJTZWEVPSR', logger=log)

audio = {
    'mic': sr.Microphone(),
    # 'voice': Client(),
    'voice': win32com.client.Dispatch("SAPI.SpVoice"),
    'speaker': pyaudio.PyAudio()
}

# Temporary database for initial testing purposes
songs = {
    'Magnet': r"C:\Users\ghoop\Desktop\PersonalAI\data\Magnet.mp3",
    'Living on the Edge': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
    'Livin on the Edge': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
    'Aerosmith': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
    'Anstatt Blumen': r"C:\Users\ghoop\Desktop\PersonalAI\data\2-02 Livin' On The Edge.m4a",
}


# Temporary wrapper to enable playing a song
def play_song(song):
    _, ext = os.path.splitext(song)
    seg = AudioSegment.from_file(song, ext[1:])

    p = audio['speaker']
    stream = p.open(format=p.get_format_from_width(seg.sample_width),
                    channels=seg.channels,
                    rate=seg.frame_rate,
                    output=True)

    # break audio into half-second chunks (to allows keyboard interrupts)
    for chunk in make_chunks(seg, 500):
        stream.write(chunk._data)

    stream.stop_stream()
    stream.close()




# Main event function which handles input and dispatching
async def run():
    rec = sr.Recognizer()

    with audio['mic'] as source:
        rec.adjust_for_ambient_noise(source)
        play_song("data\\low_beep.mp3")
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

        if intent['value'] == "play_music":
            song = None

            if 'search_query' in action:
                title = action['search_query'][0]['value']

                if title in songs:
                    song = songs[title]

            if song is None:
                song = songs['Magnet']

            log.info("PLAYING <{}>".format(song))
            play_song(song)

        else:
            voice.Speak("You said \"{}\"".format(query))

    elif 'greetings' in action:
        log.info("GREETING")
        voice.Speak("Hello")

    elif 'thanks' in action:
        log.info("THANKS")
        voice.Speak("Thanks")

    elif 'bye' in action:
        log.info("GOODBYE")
        voice.Speak("Bye")
        schedule_run = False

    else:
        log.info("Unknown Action")
        voice.Speak("I have no idea what you're saying")

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
#   Pydub: https://github.com/jiaaro/pydub
