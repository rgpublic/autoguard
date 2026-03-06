# AutoGuard

AutoGuard is a lightweight cross‑platform utility that automatically manages **AllowedIPs** in WireGuard client configurations. It solves a long‑standing limitation in WireGuard: unlike OpenVPN, WireGuard cannot push routes to clients. This makes split‑tunneling setups tedious to maintain, especially when multiple users or frequently changing routes are involved.

AutoGuard centralizes route management by fetching a simple JSON file from a predictable URL and updating the local WireGuard configuration accordingly.

## How AutoGuard Works

### Route Source Discovery

AutoGuard derives the route definition URL from the WireGuard endpoint domain.
If your WireGuard endpoint is:

wireguard.example.com

AutoGuard will request:

https://auto.wireguard.example.com/routes.json

The server must provide a JSON file with a simple key/value structure:

```json
{
  "local": "192.168.0.0/16",
  "server1": "1.2.3.4/32",
  "server2": "5.6.7.8/32"
}
```

- Keys are descriptive only.
- Values are the actual CIDR routes.
- AutoGuard merges all values into a new AllowedIPs line in the WireGuard configuration replacing the existing one.

### Configuration Update

After updating the configuration, AutoGuard ensures the new routes take effect:

- On Linux, it reimports and restarts the NetworkManager WireGuard connection.
- On Windows, it (re)installs the configuration as a WireGuard service.

## Platform Details

### Linux

* Assumes a NetworkManager‑based WireGuard setup.
* After updating the config, AutoGuard:
    * Deletes the current connection.
    * Imports the new connection to apply the new routing rules.
* Make sure your config file is named correctly, because that filename is used to name the connection.

### Windows

* AutoGuard works with the WireGuard service, which is separate from the WireGuard desktop UI.
* The UI cannot manage services installed this way.
* To control the service (start/stop/restart), you will need a helper tool such as ServiceTray:
  https://www.coretechnologies.com/products/ServiceTray/
