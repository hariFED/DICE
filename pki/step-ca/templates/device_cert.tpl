{
  "subject": {
    "commonName": "dice-device-{{ .Subject.CommonName }}"
  },
  "sans": ["dice-device-{{ .Subject.CommonName }}"],
  "keyUsage": ["digitalSignature"],
  "extKeyUsage": ["clientAuth"],
  "basicConstraints": {
    "isCA": false
  },
  "validity": {
    "notBefore": "now",
    "notAfter": "now + 87600h"
  }
}
