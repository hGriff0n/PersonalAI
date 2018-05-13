#!/usr/bin/env python3

import asyncio

from wit import Wit

from common import logger

# Immediate development work
# TODO: Create system to handle input while displaying output
# TODO: Hook up the system to my existing wit.ai work
# TODO: Ensure everything works the same as with the audio app
# TODO: Develop app to handle dispatch, forward action to that app
  # https://rpyc.readthedocs.io/en/latest/
  # https://pythonhosted.org/Pyro4/
  # http://www.zerorpc.io/

log = logger.create('audio.log')
log.setLevel(logger.logging.INFO)

client = Wit('CM7NKOIYX5BSFGPOPYFAWZDJTZWEVPSR', logger=log)


def run():
    return ()

if __name__ == "__main__":
    ()
