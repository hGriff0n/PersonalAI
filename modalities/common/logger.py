#!/usr/bin/env python3

import logging

def create(file, name=None, log_dir=None, fmt=None):
    if name is None: name = __name__
    if log_dir is None: log_dir = '.'

    hdlr = logging.FileHandler("{}/{}".format(log_dir, file))
    if fmt is None: fmt = "%(asctime)s <%(levelname)s> %(message)s"
    fmt = logging.Formatter(fmt)
    hdlr.setFormatter(fmt)

    logger = logging.getLogger(name)
    logger.addHandler(hdlr)
    return logger
