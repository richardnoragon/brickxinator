export type Data = Record<string, any>;
export let token = '';
export function uuid(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  let now = Date.now();
  for (let i = 5; i >= 0; i--) {
    bytes[i] = now % 256;
    now = Math.floor(now / 256);
  }
  bytes[6] = (bytes[6] & 15) | 112;
  bytes[8] = (bytes[8] & 63) | 128;
  const hex = [...bytes].map((b) => b.toString(16).padStart(2, '0')).join('');
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}
export async function api(
  path: string,
  method = 'GET',
  body?: unknown,
): Promise<any> {
  const form = body instanceof FormData;
  const response = await fetch(`/api/v1${path}`, {
    method,
    headers: {
      ...(method === 'GET' ? {} : { 'x-brickxinator-token': token }),
      ...(body && !form ? { 'content-type': 'application/json' } : {}),
    },
    body: body === undefined ? undefined : form ? body : JSON.stringify(body),
  });
  const value = await response
    .json()
    .catch(() => ({ error: `Request failed (${response.status})` }));
  if (!response.ok)
    throw new Error(value.error ?? `Request failed (${response.status})`);
  return value;
}
export async function bootstrap() {
  token = (await api('/bootstrap')).token;
}
export function envelope() {
  return { id: uuid(), request_key: uuid(), actor: 'local-user', reason: '' };
}
export function integer(value: string): number | null {
  if (!value.trim()) return null;
  if (!/^\d+$/.test(value.trim()))
    throw new Error('Enter a whole nonnegative number, or clear the field.');
  const number = Number(value);
  if (!Number.isSafeInteger(number))
    throw new Error(
      'The number exceeds the browser editor’s exact integer range.',
    );
  return number;
}
export function scaled(value: string, places: number): number | null {
  if (!value.trim()) return null;
  const match = value
    .trim()
    .replace(',', '.')
    .match(new RegExp(`^(\\d+)(?:\\.(\\d{0,${places}}))?$`));
  if (!match)
    throw new Error(
      `Enter a nonnegative amount with at most ${places} decimal places, or clear it.`,
    );
  return integer(match[1] + (match[2] ?? '').padEnd(places, '0'));
}
export function known(value: unknown): string {
  return typeof value === 'object' && value !== null && 'Known' in value
    ? String((value as Data).Known)
    : '';
}
export function measure(value: string, scale = 0): any {
  const number = scale ? scaled(value, scale) : integer(value);
  return number === null ? 'Unknown' : { Known: number };
}
export function money(cents: number | string | null | undefined): string {
  if (cents === null || cents === undefined) return 'Unknown';
  const n = BigInt(cents);
  return `€${n / 100n}.${String(n % 100n).padStart(2, '0')}`;
}
export function amount(cents: number | string | null | undefined): string {
  return cents === null || cents === undefined
    ? ''
    : `${BigInt(cents) / 100n}.${String(BigInt(cents) % 100n).padStart(2, '0')}`;
}
export function quantityAmount(value: string, places: number): string {
  if (!value) return '';
  const n = BigInt(value);
  const scale = 10n ** BigInt(places);
  return `${n / scale}.${String(n % scale).padStart(places, '0')}`;
}
export function rate(value: Data | null | undefined, places: number): string {
  if (!value) return 'Unknown';
  const numerator = BigInt(value.numerator);
  const denominator = BigInt(value.denominator);
  const scale = 10n ** BigInt(places);
  if (denominator === 0n) return 'Unknown';
  const scaled = numerator * scale;
  let rounded = scaled / denominator;
  if ((scaled % denominator) * 2n >= denominator) rounded++;
  return places
    ? `${rounded / scale}.${String(rounded % scale).padStart(places, '0')}`
    : String(rounded);
}
