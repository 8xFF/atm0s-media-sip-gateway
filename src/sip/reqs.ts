export async function rtpCreateOffer(
  gateway: string,
  token: string,
): Promise<{ endpoint: string; sdp: string }> {
  const res = await fetch(gateway + '/rtpengine/offer', {
    headers: {
      Accept: 'application/sdp',
      Authorization: 'Bearer ' + token,
    },
    method: 'POST',
  })

  if (res.status == 201) {
    const endpoint = res.headers.get('Location') as string
    const sdp = await res.text()
    return { endpoint, sdp }
  } else {
    const content = await res.text()
    throw new Error(content)
  }
}

export async function rtpCreateAnswer(
  gateway: string,
  sdp: string,
  token: string,
): Promise<{ endpoint: string; sdp: string }> {
  const res = await fetch(gateway + '/rtpengine/answer', {
    headers: {
      Accept: 'application/sdp',
      'Content-Type': 'application/sdp',
      Authorization: 'Bearer ' + token,
    },
    method: 'POST',
    body: sdp,
  })

  if (res.status == 201) {
    const endpoint = res.headers.get('Location') as string
    const sdp = await res.text()
    return { endpoint, sdp }
  } else {
    const content = await res.text()
    throw new Error(content)
  }
}

export async function rtpSetAnswer(endpoind: string, sdp: string) {
  const res = await fetch(endpoind, {
    headers: {
      'Content-Type': 'application/sdp',
    },
    method: 'PATCH',
    body: sdp,
  })

  if (res.status == 200) {
  } else {
    const content = await res.text()
    throw new Error(content)
  }
}

export async function rtpDelete(endpoind: string) {
  try {
    const res = await fetch(endpoind, {
      method: 'DELETE',
    })

    if (res.status == 200) {
      console.log('[RtpDelete] OK', endpoind)
    } else {
      const content = await res.text()
      console.log('[RtpDelete] Error', res.status, content)
      throw new Error(content)
    }
  } catch (e) {
    console.log('[RtpDelete] Error', e)
  }
}
