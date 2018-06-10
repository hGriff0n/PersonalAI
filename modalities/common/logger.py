#!/usr/bin/env python3

import logging

def create(file):
    hdlr = logging.FileHandler(file)
    fmt = logging.Formatter('%(asctime)s <%(levelname)s> %(message)s')
    hdlr.setFormatter(fmt)

    logger = logging.getLogger(__name__)
    logger.addHandler(hdlr)
    return logger
