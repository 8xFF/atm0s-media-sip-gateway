export function errToString(e: any) {
  if (typeof e == 'string') {
    return e
  } else if (e.to_string) {
    return e.to_string()
  } else if (e instanceof Error) {
    return (e as Error).message
  } else {
    return 'UnknownError'
  }
}
