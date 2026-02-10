export function Footer() {
  if (process.env.NODE_ENV !== "development") return null;

  return (
    <footer className="bg-[#3D3D3D] text-[#C4B8A0] px-6 md:px-12 pt-12 pb-8 mt-16">
      <div className="max-w-[1200px] mx-auto grid grid-cols-2 md:grid-cols-4 gap-8">
        <div>
          <h5 className="mb-4 text-[#E8E2D5] font-bold">About</h5>
          <a href="#mission" className="block text-[#C4B8A0] mb-2">Our Mission</a>
          <a href="#how-it-works" className="block text-[#C4B8A0] mb-2">How It Works</a>
          <a href="#contact" className="block text-[#C4B8A0] mb-2">Contact Us</a>
        </div>
        <div>
          <h5 className="mb-4 text-[#E8E2D5] font-bold">Get Involved</h5>
          <a href="#volunteer" className="block text-[#C4B8A0] mb-2">Volunteer</a>
          <a href="#submit" className="block text-[#C4B8A0] mb-2">Submit a Resource</a>
          <a href="#events" className="block text-[#C4B8A0] mb-2">Submit an Event</a>
        </div>
        <div>
          <h5 className="mb-4 text-[#E8E2D5] font-bold">Resources</h5>
          <a href="#help" className="block text-[#C4B8A0] mb-2">Find Help</a>
          <a href="#businesses" className="block text-[#C4B8A0] mb-2">Local Businesses</a>
          <a href="#calendar" className="block text-[#C4B8A0] mb-2">Event Calendar</a>
        </div>
        <div>
          <h5 className="mb-4 text-[#E8E2D5] font-bold">Information</h5>
          <a href="#privacy" className="block text-[#C4B8A0] mb-2">Privacy Policy</a>
          <a href="#accessibility" className="block text-[#C4B8A0] mb-2">Accessibility</a>
          <a href="#rights" className="block text-[#C4B8A0] mb-2">Know Your Rights</a>
        </div>
      </div>
      <div className="text-center mt-8 pt-8 border-t border-[#5D5D5D] text-[#999]">
        <p>&copy; 2026 MN Together &bull; A community resource for Minneapolis</p>
      </div>
    </footer>
  );
}
