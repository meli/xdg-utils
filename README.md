# xdg-utils

[![GitHub license](https://img.shields.io/github/license/meli/xdg-utils)](https://github.com/meli/xdg-utils/blob/master/COPYING) [![Crates.io](https://img.shields.io/crates/v/xdg-utils)](https://crates.io/crates/xdg-utils) [![docs.rs](https://docs.rs/xdg-utils/badge.svg)](https://docs.rs/xdg-utils)

Query system for default apps using XDG MIME databases.

The xdg-utils library provides dependency-free (except for `std`) Rust implementations of some common functions in the freedesktop project `xdg-utils`.


## What is implemented?
* Function `query_default_app` performs like the xdg-utils function `binary_to_desktop_file`
* Function `query_mime_info` launches the `mimetype` or else the `file` command.

Some of the utils may be implemented by combining these functions with other functions in the Rust standard library.

| Name            | Function                                               | Implemented functionalities|
|-----------------|--------------------------------------------------------|----------------------------|
|`xdg-desktop-menu`| Install desktop menu items                             | no
|`xdg-desktop-icon`| Install icons to the desktop                           | no
|`xdg-icon-resource`| Install icon resources                                 | no
|`xdg-mime`        | Query information about file type handling and install descriptions for new file types| queries only
|`xdg-open`        | Open a file or URL in the user's preferred application | all (combine crate functions with `std::process::Command`)
|`xdg-email`       | Send mail using the user's preferred e-mail composer   | no
|`xdg-screensaver` | Control the screensaver                                | no


## Specification
<https://specifications.freedesktop.org/mime-apps-spec/mime-apps-spec-latest.html>

## Reference implementation
<https://cgit.freedesktop.org/xdg/xdg-utils/tree/scripts/xdg-utils-common.in>

## Help / Feature requests/ Bugs
While this library was created for the [meli](https://meli.delivery) project, it is intended for general use. Thus you can report bugs and/or request features in the crate's repository on [github](https://github.com/meli/xdg-utils), [git.meli.delivery](https://git.meli.delivery/meli/xdg-utils) or the [meli mailing-list](https://meli.delivery/mailing-lists.html)
