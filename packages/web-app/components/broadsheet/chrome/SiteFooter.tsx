export function SiteFooter() {
  return (
    <footer className="site-footer">
      <div className="site-footer__brand">
        <h2>Minnesota, Together.</h2>
        <p>
          A community hub connecting Minnesotans with local resources, mutual aid,
          volunteer opportunities, and neighborhood events across the Twin Cities and beyond.
        </p>
        <p>
          We don&rsquo;t collect personal information&mdash;no tracking, no accounts,
          no data harvesting. If you&rsquo;d like to verify, read our{' '}
          <a href="/privacy">Privacy Policy</a> and{' '}
          <a href="/terms">Terms &amp; Conditions</a>.
        </p>
      </div>

      <div className="site-footer__columns">
        <div>
          <div className="site-footer__col-title">Community</div>
          <ul className="site-footer__links">
            <li><a href="/volunteer">Volunteer Opportunities</a></li>
            <li><a href="/mutual-aid">Mutual Aid Networks</a></li>
            <li><a href="/food-shelves">Food Shelves &amp; Pantries</a></li>
            <li><a href="/housing">Housing Resources</a></li>
            <li><a href="/warming-centers">Warming Centers</a></li>
            <li><a href="/community-gardens">Community Gardens</a></li>
          </ul>
        </div>

        <div>
          <div className="site-footer__col-title">Events</div>
          <ul className="site-footer__links">
            <li><a href="/events">Upcoming Events</a></li>
            <li><a href="/events/minneapolis">Minneapolis Events</a></li>
            <li><a href="/events/st-paul">St. Paul Events</a></li>
            <li><a href="/events/hennepin-county">Hennepin County</a></li>
            <li><a href="/events/ramsey-county">Ramsey County</a></li>
            <li><a href="/submit-event">Submit an Event</a></li>
          </ul>
        </div>

        <div>
          <div className="site-footer__col-title">Resources</div>
          <ul className="site-footer__links">
            <li><a href="/resources/emergency">Emergency Services</a></li>
            <li><a href="/resources/mental-health">Mental Health Support</a></li>
            <li><a href="/resources/legal-aid">Legal Aid</a></li>
            <li><a href="/resources/job-training">Job Training Programs</a></li>
            <li><a href="/resources/childcare">Childcare Resources</a></li>
            <li><a href="/resources/transportation">Transportation Help</a></li>
          </ul>
        </div>

        <div>
          <div className="site-footer__col-title">About</div>
          <ul className="site-footer__links">
            <li><a href="/about">About MN Together</a></li>
            <li><a href="/organizations">Partner Organizations</a></li>
            <li><a href="/contact">Contact Us</a></li>
            <li><a href="/newsletter">Newsletter</a></li>
            <li><a href="/donate">Support Our Work</a></li>
            <li><a href="/accessibility">Accessibility</a></li>
          </ul>
        </div>
      </div>

      <div className="site-footer__bottom">
        <span>&copy; 2026 Minnesota, Together</span>
        <div className="site-footer__bottom-links">
          <a href="/sitemap">SITEMAP</a>
        </div>
      </div>
    </footer>
  );
}
