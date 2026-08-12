#!/bin/env nu

def main [arg: string] {
  open $arg | from xml | to json | save /tmp/out.json
}
