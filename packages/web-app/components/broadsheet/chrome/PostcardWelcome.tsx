import Image from 'next/image';

export function PostcardWelcome() {
  return (
    <section className="postcard-welcome">
      <div className="postcard-welcome__frame">
        <Image
          src="/images/Postcard@2x.png"
          alt="A Minnesota postcard with a loon stamp"
          width={1010}
          height={690}
          priority
        />
        <div className="postcard-welcome__text">
          <span className="postcard-welcome__line">Dear neighbor,</span>
          <span className="postcard-welcome__line">Recent events reminded us what we&rsquo;re capable of.</span>
          <span className="postcard-welcome__line">Thousands showing up. Businesses closing in solidarity.</span>
          <span className="postcard-welcome__line">People offering food, shelter, rides without being asked.</span>
          <span className="postcard-welcome__line postcard-welcome__line--stanza">That doesn&rsquo;t stay in the news long. So we made this.</span>
          <span className="postcard-welcome__line postcard-welcome__line--stanza">A newspaper for what&rsquo;s next.</span>
          <span className="postcard-welcome__line">Who needs help. Who can help. Where to show up.</span>
          <span className="postcard-welcome__line">Updated weekly. Check back when you can.</span>
          <span className="postcard-welcome__line postcard-welcome__line--stanza">Next time the news fills you with dread, come here instead.</span>
          <span className="postcard-welcome__line postcard-welcome__line--stanza">
            The news moves on, <br className="mobile-br" />but we don&rsquo;t have to.
          </span>
        </div>
        <div className="postcard-welcome__addressee">
          Minnesota, together.
        </div>
      </div>
    </section>
  );
}
