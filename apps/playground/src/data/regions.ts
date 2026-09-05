export const REGION_ORDER = [
  "Global",
  "South Asia",
  "Southeast Asia",
  "East Asia",
  "North America",
  "Latin America",
  "Europe",
] as const;

const REGION_BY_COUNTRY: Record<string, (typeof REGION_ORDER)[number]> = {
  IN: "South Asia", NP: "South Asia", LK: "South Asia", BD: "South Asia", PK: "South Asia",
  TH: "Southeast Asia", VN: "Southeast Asia", SG: "Southeast Asia", MY: "Southeast Asia",
  ID: "Southeast Asia", PH: "Southeast Asia", KH: "Southeast Asia", LA: "Southeast Asia", MM: "Southeast Asia",
  HK: "East Asia", JP: "East Asia", TW: "East Asia", KR: "East Asia", CN: "East Asia",
  US: "North America", GB: "North America",
  BR: "Latin America", AR: "Latin America",
  CH: "Europe", LI: "Europe",
};

export function regionFor(countries: readonly string[]): (typeof REGION_ORDER)[number] {
  return REGION_BY_COUNTRY[countries[0] ?? ""] ?? "Global";
}

export const NON_PAYMENT_SCHEME_IDS = ["walletconnect", "otp_setup", "url"] as const;
