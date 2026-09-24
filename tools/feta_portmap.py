#!/usr/bin/env python3
"""Manage only Feta's UDP 4000 mapping on the locally discovered Archer A7 router.
No router password, DMZ, firewall disable, unrelated rule deletion or permanent lease.
"""
import argparse
import socket
import urllib.error
import urllib.request
import xml.etree.ElementTree as ET

BASE = 'http://192.168.0.1:1900'
SERVICE = 'urn:schemas-upnp-org:service:WANIPConnection:1'
DESCRIPTION = 'Feta encrypted game'
PORT = '4000'


def soap(action, fields):
    envelope = ET.Element('{http://schemas.xmlsoap.org/soap/envelope/}Envelope')
    body = ET.SubElement(envelope, '{http://schemas.xmlsoap.org/soap/envelope/}Body')
    command = ET.SubElement(body, '{' + SERVICE + '}' + action)
    for name, value in fields.items():
        ET.SubElement(command, name).text = str(value)
    req = urllib.request.Request(BASE + '/ctl/IPConn', data=ET.tostring(envelope),
        headers={'Content-Type': 'text/xml; charset="utf-8"', 'SOAPAction': '"' + SERVICE + '#' + action + '"'})
    try:
        with urllib.request.urlopen(req, timeout=5) as response:
            data = response.read(65536)
    except urllib.error.HTTPError as error:
        data = error.read(65536)
    root = ET.fromstring(data)
    values = {node.tag.rsplit('}', 1)[-1]: node.text for node in root.iter() if len(node) == 0}
    if 'errorCode' in values:
        raise RuntimeError('UPnP ' + values['errorCode'] + ': ' + str(values.get('errorDescription', '')))
    return values


def existing():
    try:
        return soap('GetSpecificPortMappingEntry', {'NewRemoteHost': '', 'NewExternalPort': PORT, 'NewProtocol': 'UDP'})
    except RuntimeError as error:
        if str(error).startswith('UPnP 714:'):
            return None
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['status', 'enable', 'remove'])
    args = parser.parse_args()
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as probe:
        probe.connect(('192.168.0.1', 9))
        local_ip = probe.getsockname()[0]
    current = existing()
    ours = current and current.get('NewPortMappingDescription') == DESCRIPTION and current.get('NewInternalPort') == PORT
    if args.action == 'status':
        print('Server LAN address:', local_ip)
        print('Public IPv4:', soap('GetExternalIPAddress', {}).get('NewExternalIPAddress'))
        print('UDP 4000:', current or 'not mapped')
        return
    if current and not ours:
        raise RuntimeError('UDP 4000 belongs to another mapping; leaving it unchanged.')
    if args.action == 'remove':
        if ours:
            soap('DeletePortMappingEntry', {'NewRemoteHost': '', 'NewExternalPort': PORT, 'NewProtocol': 'UDP'})
            print('Removed only the Feta UDP 4000 mapping.')
        return
    soap('AddPortMapping', {
        'NewRemoteHost': '', 'NewExternalPort': PORT, 'NewProtocol': 'UDP',
        'NewInternalPort': PORT, 'NewInternalClient': local_ip, 'NewEnabled': '1',
        'NewPortMappingDescription': DESCRIPTION, 'NewLeaseDuration': '3600',
    })
    result = existing()
    if not result or result.get('NewInternalClient') != local_ip or result.get('NewEnabled') != '1':
        raise RuntimeError('Router did not confirm the requested Feta mapping.')
    if result.get('NewLeaseDuration') == '0':
        soap('DeletePortMappingEntry', {'NewRemoteHost': '', 'NewExternalPort': PORT, 'NewProtocol': 'UDP'})
        raise RuntimeError('Router supplied a permanent lease; removed it instead of leaving one behind.')
    print('Confirmed UDP 4000 -> ' + local_ip + ':4000, lease ' + str(result.get('NewLeaseDuration')) + ' seconds.')


if __name__ == '__main__':
    main()
