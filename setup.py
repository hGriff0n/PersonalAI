#!/usr/bin/env python3

from setuptools import setup, find_packages

# TODO: Figure out how to get the rust exectuable built from this script
setup(name='PersonalAI',
    packages=[ 'common', 'modalities', 'modalities.plugins' ],
    url='https://github.com/hGriff0n/PersonalAI',
    description='personal ai platform',
    long_description=open('README.md').read(),
    requires=[
        'wit',
        'PyAudio',
        'SpeechRecognition',
        'pywin32',
        'pydub',
        'clg',
        'yaml',
        'anyconfig'
    ],
#     dependency_links=[
#         'https://github.com/DeepHorizons/tts/tarball/master#egg=package-1.0'
#     ],
    data_files=[('data', ['data/*'])],
    )

print("Be sure to install libav or ffmpeg for pydub to work on non-wav files")
