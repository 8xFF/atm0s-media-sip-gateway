import { parseUri, SrfRequest, SrfResponse } from 'drachtio-srf'
import { STORAGE } from 'index'

export async function handleRegister(req: SrfRequest, res: SrfResponse) {
  const contact = req.getParsedHeader('contact')
  const to = req.getParsedHeader('to')
  const expire = contact[0]?.params?.expires || req.get('expires')
  if (!expire) {
    return res.send(400, {})
  }
  const uri = parseUri(to.uri)
  const expire2 = parseInt(expire)
  const headers: { [key: string]: string } = {}
  if (expire2 > 0) {
    console.log(`On register from ${uri.user}`)
    const user = await STORAGE.getUserAuth(uri.user)
    if (user) {
      headers['Contact'] = `${req.get('Contact')};expires=${expire2}`
      const expire_at = new Date().getTime() + expire2 * 1000
      await STORAGE.setUserSession(uri.user, contact[0].uri, expire_at)
      res.send(200, {
        headers,
      })
    } else {
      res.send(404, {
        headers,
      })
    }
  } else {
    console.log(`On unregister from ${uri.user}`)
    await STORAGE.delUserSession(uri.user, contact[0].uri)
  }
}
