#!/usr/bin/env python3

import asyncio

import Pyro4

from common import logger

# Immediate development work
# TODO: Move audio playing from dispatch app into the audio app
#   Basically need to allow the dispatcher to "request" actions from its component parts
# TODO: Develop system to enable asynchronous working with non-colliding resources
#   I should still be able to type and interact with the cli app while music is playing (not for audio)
#   NOTE: This may be handled by the "server" just not handling that within the cli app (still need to handle audio restrictions)

log = logger.create('cli.log')
log.setLevel(logger.logging.INFO)



async def run():
    query = input("> ")
    log.info("TYPED <{}>".format(query))

    answer = dispatcher.dispatch(query)
    log.info("DISPATCHED <{}>".format(answer))

    print(answer['text'])
    if not answer['stop']:
        asyncio.ensure_future(run())



if __name__ == "__main__":
    dispatcher = Pyro4.Proxy("PYRONAME:ai.dispatch")
    asyncio.ensure_future(run())

    # Run until no more functions are scheduled
    while True:
        log.info("Gathering tasks")
        pending_tasks = [task for task in asyncio.Task.all_tasks() if not task.done()]
        if len(pending_tasks) == 0: break
        asyncio.get_event_loop().run_until_complete(asyncio.gather(*pending_tasks))


# API Documentation:
#   Wit: https://wit.ai/docs
