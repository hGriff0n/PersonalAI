#!/usr/bin/env python3

import asyncio

from wit import Wit

from common import logger

# Immediate development work
# TODO: Create system to handle input while displaying output
  # See if a simle "lock" would fix this (from the simple threading test - yes)
    # Single threading probably gives the same effect (yes)
    # Figure out how to implement this using coroutines
# TODO: Hook up the system to my existing wit.ai work
# TODO: Ensure everything works the same as with the audio app
# TODO: Develop app to handle dispatch, forward action to that app
  # https://rpyc.readthedocs.io/en/latest/
  # https://pythonhosted.org/Pyro4/
  # http://www.zerorpc.io/

log = logger.create('cli.log')
log.setLevel(logger.logging.INFO)

client = Wit('CM7NKOIYX5BSFGPOPYFAWZDJTZWEVPSR', logger=log)

# Pairing coroutines should work just fine
# async def get_user_input():
#     while True:
#         text = input("> ")
#         yield write_output(text)

# async def write_output(msg):
#     while await get_user_input():
#         print(msg)



def run():
    return ()

import threading
import time

lock = threading.Lock()

def message_loop():
    while True:
        time.sleep(1)
        with lock:
            print("Hello World")
            time.sleep(1)
            print("hello")

# thread = threading.Thread(target = message_loop)
# thread.start()

if __name__ == "__main__":
    print("Hello")
    time.sleep(1)
    print("World")
    input("> ")
    # while True:
    #     with lock:
    #         in_ = input("Prompt> ")
    #     time.sleep(1)
