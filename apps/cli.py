#!/usr/bin/env python3

import asyncio

from wit import Wit

from common import logger

import pyaudio
from pydub import AudioSegment
from pydub.utils import make_chunks
import os

# Immediate development work
# TODO: Develop app to handle dispatch, forward action to that app
  # https://rpyc.readthedocs.io/en/latest/
  # https://pythonhosted.org/Pyro4/
  # http://www.zerorpc.io/
# TODO: Develop system to enable asynchronous working with non-colliding resources
#   I should still be able to type and interact with the cli app while music is playing (not for audio)
#   NOTE: This may be handled by the "server" just not handling that within the cli app (still need to handle audio restrictions)

log = logger.create('cli.log')
log.setLevel(logger.logging.INFO)

client = Wit('CM7NKOIYX5BSFGPOPYFAWZDJTZWEVPSR', logger=log)
speaker = pyaudio.PyAudio()

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

    stream = speaker.open(format=speaker.get_format_from_width(seg.sample_width),
                    channels=seg.channels,
                    rate=seg.frame_rate,
                    output=True)

    # break audio into half-second chunks (to allows keyboard interrupts)
    for chunk in make_chunks(seg, 500):
        stream.write(chunk._data)

    stream.stop_stream()
    stream.close()




async def run():
    query = input("> ")
    dispatch(query)




def dispatch(query):
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
            print("You typed \"{}\"".format(query))

    elif 'greetings' in action:
        log.info("GREETING")
        print("Hello")

    elif 'thanks' in action:
        log.info("THANKS")
        print("Thanks")

    elif 'bye' in action:
        log.info("GOODBYE")
        print("Bye")
        schedule_run = False

    else:
        log.info("Unknown Action")
        print("I have no idea what you're typing")

    # Schedule another run unless we need to stop
    if schedule_run:
        asyncio.ensure_future(run())



if __name__ == "__main__":
    asyncio.ensure_future(run())

    # Run until no more functions are scheduled
    while True:
        log.info("Gathering tasks")
        pending_tasks = [task for task in asyncio.Task.all_tasks() if not task.done()]
        if len(pending_tasks) == 0: break
        asyncio.get_event_loop().run_until_complete(asyncio.gather(*pending_tasks))


# API Documentation:
#   Wit: https://wit.ai/docs
