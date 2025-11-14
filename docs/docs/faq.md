---
sidebar_position: 4
---

# 📖 Frequently Asked Questions

This page includes several frequently asked questions and explanations.

All of these answers are written from the perspective of the project owner.

**Last Update:** 10th May 2024

<details>
  <summary>Why another project?</summary>

  I think this is best explained with a bit of history:
  - Stoat (formerly Revolt) started as a passion project
  - It grew way beyond any of our expectations
  - We might as well keep going since there is an interest in this space

  Beyond that:
  - Stoat has been a great learning experience, including development, management, and running the infrastructure for a large project. Stoat has also taught me a lot about different concepts and programming languages, and really, that's how developers learn. We make cool projects to try to better how we work, it doesn't matter if someone has done it before as long as you can attempt to do the same. Reinventing the wheel is part of the process.
  - At the time, there was also a relative void of competition in this specific genre of chat platforms. There were Guilded, Discord, and Matrix but these are all either closed-source or cater to a different audience.

  PS. I've had a few people say, 'why not just contribute to X?', the answer is quite simple, I just didn't know about any of these projects (i.e. Matrix).
</details>

<details>
  <summary>How are we funded?</summary>

  Stoat is entirely funded through donations, we have amassed a significant amount of money from donations alone. (financial transparency reports coming soon :tm:)

  The month-to-month income of Stoat covers our operational costs and leaves enough spare to cover yearly expenses and the occassional one-time expense, such as for additional hardware.

  We have monetisation plans lined up for the future, however it is not our intention to paywall existing features, instead where possible we intend to pass down costs such as for file storage or voice bandwidth.
</details>

<details>
  <summary>'X' feature when?</summary>

  Please take a look at [our roadmap on GitHub](https://op.stoatinternal.com/projects/all-of-revolt/gantt?query_id=53).
</details>

<details>
  <summary>Does Stoat have federation?</summary>

  As of right now, Stoat does not feature any federation and **it is not in our feature roadmap**.

  However, this does not necessarily mean federation is off the table, possible avenues are:
  - Implement our own federation protocol
  - Implement a promising up and coming federation protocol, polyproto
  - Implement the Matrix protocol (unlikely, obtuse and unstable)
  - Implement the XMPP protocol (battle-tested and stable)

  Any federation that is implemented MUST exercise caution in:
  - Preventing spam and abuse: moderators should be able to block malicious actors
  - Protecting user data: users should be able to redact all of their information and messages
</details>

<details>
  <summary>What can I do with Stoat and how do I self-host?</summary>

  In general:
  - The Stoat branding is used to represent the platform, stoat.chat.
  - You may use the branding as-is to promote the platform and your community on the platform.
  - You should not use the branding in order to appear as if you are associated with the Stoat team.
  - Please make explicit distinctions between Stoat (the platform, "stoat.chat") and the Stoat software.
  - The Stoat software is provided unbranded and only associated by name.

  If you have any concerns or questions, please liase with us at [contact@stoat.chat](mailto:contact@stoat.chat).

  As a third-party platform:
  - You **must** provide correct attribution in line with our software licenses:

    If you are using official images (GitHub Packages / Docker Hub), attribution is included.
    If you are modifying the software and using it in production, you must publish the changes to the source publicly in line with AGPLv3. (In addition to providing attribution back to the original project.)
  - You are **solely responsible** for whatever happens on your third party instance, we provide no warranty or liability for what happens on 3rd party instances.
  - You **must not** appear to associate with Stoat / stoat.chat unless if granted explicit written permission. In regards to custom clients, provide a warning of any potential risks or clear it with us.
  - You **may not** use any of the Stoat branding or brand assets to advertise or promote your third party instance.

  You can self-host Stoat by:
  - Using [Docker Compose and our recommended guide](https://github.com/stoatchat/self-hosted).
  - Building individual components yourself from the [source code](https://github.com/stoatchat).
</details>

<details>
  <summary>Can you verify my server/bot?</summary>

  Currently, you can only apply to verify servers given that you have a valid reason to believe verification is necessary for your community. Verification is intended to provide protection for server owners from copy cats and to provide authenticity to users as such we are not just giving it out to anyone because that would defeat the purpose.

  However if you would like to get a server verified, you should satisfy one of the following criteria:

  - Official community for a well-established open source project
  - Official community for any other well-established product, service, or person
  - Large and active distinct pre-existing community

    Distinct means the community is unique and well-known (& has an active presence) off platform. This means we are not currently verifying generic servers that centre around a topic unless if it meets one of the first two criteria. Though in special circumstances, well known on platform communities may also be considered.

  Server verification also comes with a vanity invite, so please have one ready if you want to apply. To apply, drop an email at [contact@stoat.chat](mailto:contact@stoat.chat).

  We also periodically prune verification from servers that have fallen into disrepair and / or otherwise are no longer active.
</details>

For questions about the Stoat platform, you may want to go to our knowledge base:

- [What badges can I get?](https://support.stoat.chat/kb/account/badges)
- [How old do I have to be to use Stoat?](https://support.stoat.chat/kb/safety/minimum-age-guidelines)
- [Are there any restrictions on servers being on Discover?](https://support.stoat.chat/kb/safety/discover-guidelines)
- [(... and more)](https://support.stoat.chat)
