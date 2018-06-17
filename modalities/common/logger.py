#!/usr/bin/env python3

import logging

def create(file, name=None):
    hdlr = logging.FileHandler(file)
    fmt = logging.Formatter('%(asctime)s <%(levelname)s> %(message)s')
    hdlr.setFormatter(fmt)

    if name is None: name = __name__
    logger = logging.getLogger(name)
    logger.addHandler(hdlr)
    return logger
