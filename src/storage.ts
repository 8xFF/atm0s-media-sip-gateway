export class SipDatabase {
  users: { [key: string]: string } = {
    1001: 'pass1',
    1002: 'pass2',
    1003: 'pass3',
    1004: 'pass4',
    1005: 'pass5',
  }
  sessions = new Map<string, Map<string, number>>()

  async getUserAuth(number: string): Promise<string | undefined> {
    console.log(`Get auth for ${number} => `, !!this.users[number])
    return this.users[number]
  }

  async setUserSession(number: string, dest: string, expire: number) {
    const user = this.sessions.get(number) || new Map()
    if (!user.has(dest)) {
      if (user.size == 0) {
        console.log(`User ${number} added first dest ${dest}`)
      } else {
        console.log(`User ${number} added other dest ${dest}`)
      }
    }
    user.set(dest, expire)
    this.sessions.set(number, user)
  }

  async delUserSession(number: string, dest: string) {
    const user = this.sessions.get(number) || new Map()
    if (user.has(dest)) {
      console.log(`User ${number} removed dest ${dest}`)
    }
    user.delete(dest)
    if (user.size == 0) {
      this.sessions.delete(number)
      console.log(`User ${number} removed all dests`)
    }
  }

  async getUserDests(number: string): Promise<string[]> {
    const user = this.sessions.get(number) || new Map<string, number>()
    const dests: string[] = []
    user.forEach((value, key) => {
      dests.push(key)
    })
    return dests
  }
}
