import SwiftUI

struct ContentView: View {
    @State private var text: String = ""
    @State private var boxes: [String] = []
    @State private var focusedBox: Int?
    @State private var scrollToBottom = false
    
    var body: some View {
        VStack {
            ScrollViewReader { scrollViewProxy in
                ScrollView {
                    VStack(spacing: 10) {
                        ForEach(boxes.indices, id: \.self) { index in
                            Text(boxes[index])
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding()
                                .background(
                                    RoundedRectangle(cornerRadius: 8)
                                        .stroke(lineWidth: index == focusedBox ? 6.0 : 2.5)
                                        .foregroundColor(index == focusedBox ? .blue : .blue)
                                )
                                .id(index)
                                .onTapGesture {
                                    focusedBox = index
                                }
                        }
                        .onChange(of: scrollToBottom) { value in
                            if value {
                                scrollViewProxy.scrollTo(boxes.count - 1, anchor: .bottom)
                                scrollToBottom = false
                            }
                        }
                    }
                }
                .padding()
                .onAppear {
                    scrollToBottom = true
                }
            }
            
            TextField("Enter Your Command", text: $text)
                .onSubmit(addBox)
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(Color.blue, lineWidth: 1)
                        .background(Color.clear)
                )
                .frame(maxWidth: .infinity, minHeight: 40)
                .padding(.horizontal, 12)
                .background(Color.clear)
        }
        .frame(minWidth: 400, minHeight: 300)
        .background(Color(NSColor.windowBackgroundColor))
        .onReceive(NotificationCenter.default.publisher(for: NSApplication.willTerminateNotification)) { _ in
            focusedBox = nil
        }
    }
    
    func getOutputForCommand() {
        guard let url = URL(string: "http://127.0.0.1:8000/get_output") else {
            return
        }
        
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        let body: [String: AnyHashable] = ["command": text]
        request.httpBody = try? JSONSerialization.data(withJSONObject: body, options: .fragmentsAllowed)
        let task = URLSession.shared.dataTask(with: request) { data, _, error in
            guard let data = data, error == nil else {
                return
            }
            do {
                let response = String(data: data, encoding: .utf8)
                if let unwrappedResponse = response {
                    boxes.append(unwrappedResponse)
                    scrollToBottom = true
                    if let firstCharacter = text.first{
                        if firstCharacter == "#"{
                            text = unwrappedResponse.split(separator: "\n").dropFirst().joined(separator: "\n").removingPercentEncoding!
                        } else{
                            text = ""
                        }
                    }
                } else {
                    print("No Response")
                }
            }
        }
        task.resume()
    }
    
    func addBox() {
        if !text.isEmpty {
            getOutputForCommand()
            focusedBox = boxes.count - 1
        }
    }
}

struct CommandResponse: Codable {
    let output: String
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}

enum CommandError: Error {
    case invalidURL
    case invalidResponse
    case invalidData
}
