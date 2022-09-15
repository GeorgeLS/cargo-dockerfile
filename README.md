# cargo dockerfile

## Overview
cargo dockerfile is a cargo plugin which allows you to create an optimal Dockerfile out of your cargo project.\
The meaning of optimal here is within the context of build time. The resulting Dockerfile will create steps for building\
each of your crates within the project separately, leveraging the docker layers. In that way, the first time you build\
your project it may take more than it would by just running **cargo build** at the root of your project but subsequent builds\
will take significantly less time since pretty much everything will be cached (based on what parts of the code you change every time).\
cargo dockerfile will take care of any dependencies between your libaries and binaries so that everything is build in the correct order.

## Disclaimer

This plugin will allow you to **build** your cargo project but it doesn't guarantee that it will **run** as well.\
Anything your program(s) need in order to run, you need to add those to the Dockerfile yourself (e.g specific command line arguments, env variables, etc).

## Notes

- You should run this in the root of your project
- If you have existing Dockerfile in the root of your project, then it will **not** be overwritten. Instead, a new Dockerfile with name **cargo-dockerfile.Dockerfile** will be created

## Command line options
You can specify some command line options so that the result Dockerfile is more *complete*:

- -a, --app-path: This is the path where all of your binaries will be stored inside the docker image. Default: **/app**
- -b, --builder-image: This is the image that will be used to build your project in the form of [image]:[tag]. Default: **rust:latest**
- -c, --cmd: This is a string containing the command that will be used with **CMD** dockerfile command. Default: None
- -r, --runner-image: This is the image that will be used as a base to run your program(s). If not specified then your programs will reside inside the builder image in the **app-path** specified. Default: None
- -u, --user: The user that will be created for the docker image. Default: Current user logged-in in host machine
