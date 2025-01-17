# SecureDash-ESP32

SecureDash-ESP is an ESP32-based wireless accessible Tesla USB drive written in Rust.
The author made a mistake and forgot to check the possible read/write speed limit of the SD card and USB; as a result, **it cannot sustain the needs of its original purpose**.
Despite that, it might still be educational and potentially useful for some use cases that don't require high-speed IOs.
Therefore, we open-sourced it regardless.
Please read the article [ESP32 Tesla dashcam remote USB project in Rust failed. Here's what I've learned](https://fangpenlin.com/posts/2025/01/17/my-rust-esp32-project-failure/) to learn more about the story of this project.
We won't provide any updates or bug fixing for this project at this moment.

# Hardware

We developed this system with [ESP32-S3-USB-OTG](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32s3/esp32-s3-usb-otg/user_guide.html).
You can purchase it [here from Amazon](https://www.amazon.com/Espressif-ESP32-S3-USB-OTG-Development-Board/dp/B09JZ8PTLX) (not an affiliate link).

# Config file
We designed this wireless USB thumb drive so Tesla can be as easily configurable as possible.
We plan to provide a web UI for generating a simple TOML config file for the users to drop into the USB drive to configure it.
Therefore, even non-technical users should be able to use it.

The config file filename is `securedash.toml`, and there are a few sections in it.

## Wifi

The Wifi section defines how it should connect to the home Wifi.
Here's an example:

```TOML
[wifi]
ssid = "my-home-wifi"
password = "my-super-duper-secret-password"
auth_method = "WPA3Personal"
```

Please note that the `auth_method` is optional.
By default, `WPA2Personal` will be used if it is not provided.
The all available `auth_method` options can be found [here](https://github.com/LaunchPlatform/securedash-esp32/blob/cff762a9cd502c62caabc0c75c4b9111c88bac02/src/config.rs#L7-L17).

## API

The API section defines which websocket endpoint to connect to when Wifi connection is available.
Here's an example:

```
[api]
endpoint = "ws://192.168.100.123:8080/tesla-backup"
```

# API

We envisioned the storage server always running in the home network or on a public endpoint.
The Telsa vehicle returns and connects to the home Wifi without a well-known IP address.
Therefore, making it a simple HTTP API server running on ESP32 is unsuitable for this use case.
Instead, we make it a simple Websocket client connecting to a known endpoint of the storage server.
Upon connection, the storage server can send request frames to the ESP32 via WebSocket.
The commands are in simple JSON format like this:

```json
{
    "id": "unique_id_of_req",
    "command": {"PAYLOAD OF COMMAND": "..."}
}
```

Here are the available commands.

## GetInfo

Request asking ESP32 for its device information.
For example:

```json
{
    "id": "a62fdfb7-4aed-413d-953d-ed3b54cce2b3",
    "command": {
        "type": "GetInfo"
    }
}
```

## ListFiles

Request that ESP32 list files on a specific path.
For example:

```json
{
    "id": "222e46a8-bc3e-4867-84aa-b47d3beae193",
    "command": {
        "type": "ListFiles",
        "path": "/disk"
    }
}
```

## FetchFile

Request that ESP32 fetch content and return in multiple binary frames.
For example:

```json
{
    "id": "222e46a8-bc3e-4867-84aa-b47d3beae193",
    "command": {
        "type": "FetchFile",
        "path": "/disk/TeslaCam/RecentClips/2023-11-15_14-02-02-back.mp4",
        "chunk_size": 4096
    }
}
```

## Reboot

Request that ESP32 reboot itself.
For example:


```json
{
    "id": "a62fdfb7-4aed-413d-953d-ed3b54cce2b3",
    "command": {
        "type": "Reboot"
    }
}
```

Not implemented yet.
